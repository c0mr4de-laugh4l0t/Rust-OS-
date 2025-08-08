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

/// Initialize the global PMM using the fallback pool.
/// Safety: caller must ensure physical == virtual identity mapping for the addresses used,
/// or that the bitmap region is accessible via current virtual mapping.
unsafe fn pmm_setup_fallback() {
    // compute total frames manageable in fallback pool
    let total_frames = FALLBACK_POOL_SIZE / FRAME_SIZE;
    let bitmap_bytes_needed = (total_frames + 7) / 8;
    // round bitmap storage up to whole frames
    let bitmap_frames = round_up_frames(bitmap_bytes_needed) ;
    let bitmap_bytes_reserved = bitmap_frames * FRAME_SIZE;

    // choose bitmap physical address at start of pool
    let bitmap_phys = FALLBACK_POOL_START;
    let bitmap_ptr = bitmap_phys as *mut u8;

    // choose base_frame to be after bitmap region
    let base_frame_addr = FALLBACK_POOL_START + bitmap_bytes_reserved;
    let base_frame = base_frame_addr / FRAME_SIZE;
    let frames_managed = (FALLBACK_POOL_SIZE - bitmap_bytes_reserved) / FRAME_SIZE;

    // Initialize PMM global
    PMM.init(bitmap_ptr, bitmap_bytes_reserved, base_frame, frames_managed);

    // mark the bitmap pages as used so they won't be allocated
    for i in 0..bitmap_frames {
        let pa = FALLBACK_POOL_START + i * FRAME_SIZE;
        PMM.mark_used(pa);
    }

    // (Optional) if you have kernel start/end linker symbols, mark kernel pages used here:
    // extern "C" { static __kernel_start: u8; static __kernel_end: u8; }
    // let kstart = &__kernel_start as *const _ as usize;
    // let kend = &__kernel_end as *const _ as usize;
    // for pa in (kstart & !(FRAME_SIZE-1) ..= kend).step_by(FRAME_SIZE) { PMM.mark_used(pa); }

    // Print PMM stats to VGA
    crate::vga::vprintln!("PMM initialized: frames_managed={}, free_frames={}", frames_managed, PMM.free_frames());

    // Quick test: allocate 10 frames and free them.
    {
        let mut allocated: [usize; 16] = [0; 16];
        for i in 0..10 {
            if let Some(f) = PMM.alloc_frame() {
                crate::vga::vprintln!("alloc {} -> 0x{:x}", i, f.start_address());
                allocated[i] = f.start_address();
            } else {
                crate::vga::vprintln!("alloc {} -> None", i);
            }
        }
        crate::vga::vprintln!("Free frames after alloc: {}", PMM.free_frames());
        // free them
        for i in 0..10 {
            let pa = allocated[i];
            if pa != 0 {
                PMM.free_frame(pa);
                crate::vga::vprintln!("freed 0x{:x}", pa);
            }
        }
        crate::vga::vprintln!("Free frames after free: {}", PMM.free_frames());
    }
    }
