#![no_std]
#![no_main]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod interrupts;
mod vga;
mod kb;
mod memory;
mod scheduler;
mod fs;

use vga::VgaWriter;
use crate::vga::VGA_WRITER;
use crate::kb::Kb;
use memory::{PhysicalMemoryManager, FRAME_SIZE};

entry_point!(kernel_main);

static mut PMM: PhysicalMemoryManager = PhysicalMemoryManager::new_uninit();

fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    interrupts::init_idt();
    interrupts::remap_pic();
    interrupts::enable_interrupts();

    {
        let mut vw = VGA_WRITER.lock();
        vw.clear_screen();
        vw.write_str("\n=== IronVeil / Nexis (VGA IRQ keyboard + PMM + Scheduler + FS) ===\n");
        vw.write_str("On-screen console ready. Type 'help'.\n\n");
    }

    crate::vga::sprintln!("\n=== IronVeil / Nexis (serial) ===");
    crate::vga::sprintln!("IRQ keyboard active. Type 'help' and press Enter.\n");

    unsafe {
        pmm_setup_linker();
    }

    fs::fs_init();
    Kb::init();

    extern "C" fn demo_task() {
        let mut i: u64 = 0;
        loop {
            crate::vga::vprintln!("demo_task: tick {}", i);
            i = i.wrapping_add(1);
            for _ in 0..200_000 { core::hint::spin_loop(); }
            crate::scheduler::task_yield();
        }
    }

    extern "C" fn shell_task() {
        shell_loop();
    }

    unsafe {
        if let Some(slot) = crate::scheduler::spawn(demo_task, &PMM, 4) {
            crate::vga::vprintln!("Spawned demo task at slot {}", slot);
        }
        if let Some(slot) = crate::scheduler::spawn(shell_task, &PMM, 16) {
            crate::vga::vprintln!("Spawned shell task at slot {}", slot);
        }
    }

    crate::scheduler::schedule_loop()
}

fn shell_loop() -> ! {
    use kb::Kb;
    let mut rng = crate::kb::XorShift64::new(0xabcdef123456789u64);

    loop {
        crate::vga::vprint!("ironveil@nexis:~$ ");
        crate::vga::sprint!("ironveil@nexis:~$ ");

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
                crate::vga::vprintln!("  fs ls      - list files");
                crate::vga::vprintln!("  fs cat <file> - print file contents");
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
            x if x.starts_with("fs ls") => {
                crate::fs::list_files();
            }
            x if x.starts_with("fs cat ") => {
                let parts: Vec<&str> = x.splitn(3, ' ').collect();
                if parts.len() == 3 {
                    crate::fs::print_file(parts[2]);
                } else {
                    crate::vga::vprintln!("Usage: fs cat <filename>");
                }
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

unsafe fn pmm_setup_linker() {
    extern "C" {
        static __kernel_start: u8;
        static __kernel_end: u8;
    }
    let kstart = &__kernel_start as *const _ as usize;
    let kend = &__kernel_end as *const _ as usize;
    let kernel_end_page = ((kend + FRAME_SIZE - 1) / FRAME_SIZE) * FRAME_SIZE;
    let pool_start = if kernel_end_page < 0x0010_0000 { 0x0010_0000 } else { kernel_end_page };
    let max_pool_size = 128 * 1024 * 1024usize;
    let pool_end = pool_start.saturating_add(max_pool_size);
    let pool_size = if pool_end > pool_start { pool_end - pool_start } else { 0usize };
    if pool_size < FRAME_SIZE {
        crate::vga::vprintln!("PMM(linker): not enough pool after kernel, falling back.");
        pmm_setup_fallback();
        return;
    }
    let total_frames = pool_size / FRAME_SIZE;
    let bitmap_bytes_needed = (total_frames + 7) / 8;
    let bitmap_frames = (bitmap_bytes_needed + FRAME_SIZE - 1) / FRAME_SIZE;
    let bitmap_phys = pool_start;
    let bitmap_bytes_reserved = bitmap_frames * FRAME_SIZE;
    let base_frame_addr = pool_start + bitmap_bytes_reserved;
    let base_frame = base_frame_addr / FRAME_SIZE;
    let frames_managed = (pool_size - bitmap_bytes_reserved) / FRAME_SIZE;
    PMM.init(bitmap_phys as *mut u8, bitmap_bytes_reserved, base_frame, frames_managed);
    for i in 0..bitmap_frames {
        let pa = bitmap_phys + i * FRAME_SIZE;
        PMM.mark_used(pa);
    }
    let kstart_frame = (kstart / FRAME_SIZE) * FRAME_SIZE;
    let kend_frame = ((kend + FRAME_SIZE - 1) / FRAME_SIZE) * FRAME_SIZE;
    let mut pa = kstart_frame;
    while pa < kend_frame {
        PMM.mark_used(pa);
        pa += FRAME_SIZE;
    }
    crate::vga::vprintln!(
        "PMM(linker): kernel:0x{:x}-0x{:x}, pool 0x{:x}-0x{:x}, frames_managed={}",
        kstart, kend, pool_start, pool_start + pool_size, frames_managed
    );
}

unsafe fn pmm_setup_fallback() {
    const FALLBACK_POOL_START: usize = 0x0010_0000;
    const FALLBACK_POOL_SIZE: usize = 64 * 1024 * 1024;
    let total_frames = FALLBACK_POOL_SIZE / FRAME_SIZE;
    let bitmap_bytes_needed = (total_frames + 7) / 8;
    let bitmap_frames = (bitmap_bytes_needed + FRAME_SIZE - 1) / FRAME_SIZE;
    let bitmap_bytes_reserved = bitmap_frames * FRAME_SIZE;
    let bitmap_phys = FALLBACK_POOL_START;
    let bitmap_ptr = bitmap_phys as *mut u8;
    let base_frame_addr = FALLBACK_POOL_START + bitmap_bytes_reserved;
    let base_frame = base_frame_addr / FRAME_SIZE;
    let frames_managed = (FALLBACK_POOL_SIZE - bitmap_bytes_reserved) / FRAME_SIZE;
    PMM.init(bitmap_ptr, bitmap_bytes_reserved, base_frame, frames_managed);
    for i in 0..bitmap_frames {
        let pa = FALLBACK_POOL_START + i * FRAME_SIZE;
        PMM.mark_used(pa);
    }
    crate::vga::vprintln!("PMM(fallback): frames_managed={}, free_frames={}", frames_managed, PMM.free_frames());
}