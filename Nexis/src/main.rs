#![no_std]
#![no_main]

// Nexis: IRQ keyboard (low-level PIC + IDT) + VGA shell + PMM init (fallback pool)
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod interrupts;
mod vga;
mod kb;
mod memory; // <- PMM module you added

use vga::VgaWriter;
use crate::vga::VGA_WRITER;
use crate::kb::Kb;
use memory::{PhysicalMemoryManager, PhysFrame, FRAME_SIZE};

entry_point!(kernel_main);

// Global PMM instance (uninitialized until pmm_setup)
static mut PMM: PhysicalMemoryManager = PhysicalMemoryManager::new_uninit();

fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    // init IDT + PIC + enable interrupts
    interrupts::init_idt();
    interrupts::remap_pic();
    interrupts::enable_interrupts();

    // init VGA & clear
    {
        let mut vw = VGA_WRITER.lock();
        vw.clear_screen();
        vw.write_str("\n=== IronVeil / Nexis (VGA IRQ keyboard + PMM) ===\n");
        vw.write_str("On-screen console ready. Type 'help'.\n\n");
    }

    // Print to serial too (optional)
    crate::vga::sprintln!("\n=== IronVeil / Nexis (serial) ===");
    crate::vga::sprintln!("IRQ keyboard active. Type 'help' and press Enter.\n");

    // === Initialize Physical Memory Manager (fallback) ===
    unsafe {
        pmm_setup_fallback();
    }

    // initialize keyboard queue
    Kb::init();

    // Run the exact same shell logic as before, but read keys from kb queue
    shell_loop()
}

/// Shell loop: uses kb::Kb::read_line_irq() to get lines (no polling)
fn shell_loop() -> ! {
    use kb::Kb;
    use core::str;

    let mut rng = crate::kb::XorShift64::new(0xabcdef123456789u64);

    loop {
        crate::vga::vprint!("ironveil@nexis:~$ ");
        crate::vga::sprint!("ironveil@nexis:~$ ");

        // read a line (blocking, uses hlt while waiting)
        let line = Kb::read_line_irq();

        let cmd = line.trim();
        match cmd {
            "help" => {
                crate::vga::vprintln!("Available commands:");
                crate::vga::vprintln!("  help       - this message");
                crate::vga::vprintln!("  clear|cls  - clear screen");
                crate::vga::vprintln!("  genpass    - generate a 16-char password");
                crate::vga::vprintln!("  ip         - fake IPv4");
                crate::vga::vprintln!("  mac        - fake MAC");
                crate::vga::vprintln!("  reboot     - halt (restart QEMU)");
            }
            "clear" | "cls" => {
                VGA_WRITER.lock().clear_screen();
            }
            "genpass" => {
                let mut pass = [0u8; 16];
                for i in 0..16 {
                    let b = rng.next_range_u8(33u8, 126u8);
                    pass[i] = b;
                }
                let p = unsafe { core::str::from_utf8_unchecked(&pass) };
                crate::vga::vprintln!("Generated password: {}", p);
            }
            "ip" => {
                let a = rng.next_range_u8(10, 250);
                let b = rng.next_range_u8(1, 254);
                let c = rng.next_range_u8(1, 254);
                let d = rng.next_range_u8(1, 254);
                crate::vga::vprintln!("Fake IPv4: {}.{}.{}.{}", a, b, c, d);
            }
            "mac" => {
                let mut parts = [0u8; 6];
                for i in 0..6 { parts[i] = rng.next_u8(); }
                crate::vga::vprintln!("Fake MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]);
            }
            "reboot" => {
                crate::vga::vprintln!("Reboot requested — halting kernel (restart QEMU).");
                loop { core::hint::spin_loop(); }
            }
            "" => {}
            _ => crate::vga::vprintln!("Unknown command: '{}'. Type 'help'.", cmd),
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::vga::vprintln!("\n\n*** PANIC ***");
    if let Some(loc) = info.location() {
        crate::vga::vprintln!("panic at {}:{}: {}", loc.file(), loc.line(), info);
    } else {
        crate::vga::vprintln!("panic: {}", info);
    }
    loop { core::hint::spin_loop(); }
}

// ---------------------- PMM fallback setup + test ------------------------

/// Fallback pool: start at 1 MiB, use 64 MiB pool for the PMM bitmap + frames.
/// This is safe for QEMU/dev testing. Adjust if you know your real memory map.
const FALLBACK_POOL_START: usize = 0x0010_0000; // 1 MiB
const FALLBACK_POOL_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

/// Round up to FRAME_SIZE frames
#[inline]
fn round_up_frames(bytes: usize) -> usize {
    (bytes + FRAME_SIZE - 1) / FRAME_SIZE
}

// Linker-based PMM setup: place bitmap just after the kernel image.
unsafe fn pmm_setup_linker() {
    extern "C" {
        static __kernel_start: u8;
        static __kernel_end: u8;
    }

    // Get kernel start/end physical addresses (these are linker symbols).
    let kstart = &__kernel_start as *const _ as usize;
    let kend = &__kernel_end as *const _ as usize;

    // Align end to next frame so bitmap doesn't overlap kernel end
    let kernel_end_page = ((kend + FRAME_SIZE - 1) / FRAME_SIZE) * FRAME_SIZE;

    // Decide how much memory to use for PMM: use rest of first 64 MiB region after kernel end,
    // but you can tune this to use entire RAM by reading BootInfo later.
    // Here we take a conservative default pool of up to 128 MiB minus kernel_end_page.
    let pool_start = kernel_end_page;
    // make sure pool_start is at least 1 MiB
    let pool_start = if pool_start < 0x0010_0000 { 0x0010_0000 } else { pool_start };
    let max_pool_size = 128 * 1024 * 1024usize; // 128 MiB pool cap for safety
    let pool_end = pool_start.saturating_add(max_pool_size);

    // Compute frames in pool
    let pool_size = if pool_end > pool_start { pool_end - pool_start } else { 0 };
    if pool_size < FRAME_SIZE {
        crate::vga::vprintln!("PMM: not enough pool memory after kernel (pool_size={})", pool_size);
        // fallback to earlier fallback routine
        pmm_setup_fallback();
        return;
    }

    let total_frames = pool_size / FRAME_SIZE;
    let bitmap_bytes_needed = (total_frames + 7) / 8;
    // reserve whole frames for the bitmap itself
    let bitmap_frames = (bitmap_bytes_needed + FRAME_SIZE - 1) / FRAME_SIZE;
    let bitmap_phys = pool_start;
    let bitmap_bytes_reserved = bitmap_frames * FRAME_SIZE;

    let base_frame_addr = pool_start + bitmap_bytes_reserved;
    let base_frame = base_frame_addr / FRAME_SIZE;
    let frames_managed = (pool_size - bitmap_bytes_reserved) / FRAME_SIZE;

    // initialize PMM
    PMM.init(bitmap_phys as *mut u8, bitmap_bytes_reserved, base_frame, frames_managed);

    // mark bitmap pages used
    for i in 0..bitmap_frames {
        let pa = bitmap_phys + i * FRAME_SIZE;
        PMM.mark_used(pa);
    }

    // mark kernel pages used
    let kstart_frame = (kstart / FRAME_SIZE) * FRAME_SIZE;
    let kend_frame = ((kend + FRAME_SIZE - 1) / FRAME_SIZE) * FRAME_SIZE;
    let mut pa = kstart_frame;
    while pa < kend_frame {
        PMM.mark_used(pa);
        pa += FRAME_SIZE;
    }

    crate::vga::vprintln!(
        "PMM(linker): kernel:0x{:x}-0x{:x}, pool 0x{:x}-0x{:x}, frames={}",
        kstart, kend, pool_start, pool_start + pool_size, frames_managed
    );

    // quick test like before
    {
        let mut allocated: [usize; 8] = [0; 8];
        for i in 0..8 {
            if let Some(f) = PMM.alloc_frame() {
                crate::vga::vprintln!("alloc {} -> 0x{:x}", i, f.start_address());
                allocated[i] = f.start_address();
            }
        }
        crate::vga::vprintln!("Free after alloc: {}", PMM.free_frames());
        for i in 0..8 {
            let pa = allocated[i];
            if pa != 0 {
                PMM.free_frame(pa);
                crate::vga::vprintln!("freed 0x{:x}", pa);
            }
        }
        crate::vga::vprintln!("Free after free: {}", PMM.free_frames());
    }
}
