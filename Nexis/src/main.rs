#![no_std]
#![no_main]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod interrupts;
mod vga;
mod kb;
mod memory; // PMM module
mod scheduler; // scheduler & spawn API (from Phase 2)

use vga::VgaWriter;
use crate::vga::VGA_WRITER;
use crate::kb::Kb;
use memory::{PhysicalMemoryManager, FRAME_SIZE};

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
        vw.write_str("\n=== IronVeil / Nexis (VGA IRQ keyboard + PMM + Scheduler) ===\n");
        vw.write_str("On-screen console ready. Type 'help'.\n\n");
    }

    // Print to serial too (optional)
    crate::vga::sprintln!("\n=== IronVeil / Nexis (serial) ===");
    crate::vga::sprintln!("IRQ keyboard active. Type 'help' and press Enter.\n");

    // === Initialize Physical Memory Manager (linker-based, with fallback) ===
    unsafe {
        pmm_setup_linker();
    }

    // initialize keyboard queue
    Kb::init();

    // === Spawn tasks: make shell a task + a demo task to prove multitasking ===
    // demo_task: quick ticking test that yields
    extern "C" fn demo_task() {
        let mut i: u64 = 0;
        loop {
            crate::vga::vprintln!("demo_task: tick {}", i);
            i = i.wrapping_add(1);
            // small busy-wait to simulate work, then yield
            for _ in 0..200_000 { core::hint::spin_loop(); }
            crate::scheduler::task_yield();
        }
    }

    // shell_task: wrapper that calls your existing shell_loop (which never returns)
    extern "C" fn shell_task() {
        // call into the existing interactive shell — it blocks and never returns normally
        shell_loop();
    }

    // Spawn demo task (4 pages stack) and shell task (16 pages stack)
    // spawn signature: spawn(entry: extern "C" fn(), pmm: &PhysicalMemoryManager, pages: usize) -> Option<usize>
    unsafe {
        if let Some(slot) = crate::scheduler::spawn(demo_task, &PMM, 4) {
            crate::vga::vprintln!("Spawned demo task at slot {}", slot);
        } else {
            crate::vga::vprintln!("Failed to spawn demo_task");
        }

        if let Some(slot) = crate::scheduler::spawn(shell_task, &PMM, 16) {
            crate::vga::vprintln!("Spawned shell task at slot {}", slot);
        } else {
            crate::vga::vprintln!("Failed to spawn shell_task");
        }
    }

    // start scheduler (never returns)
    crate::scheduler::schedule_loop()
}

/// Shell loop: uses kb::Kb::read_line_irq() to get lines (no polling)
fn shell_loop() -> ! {
    use kb::Kb;

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
                crate::vga::vprintln!("  pmmstat    - show PMM stats (debug)");
                crate::vga::vprintln!("  spawndemo  - spawn another demo task");
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
                crate::vga::vprintln!(
                    "Spoofed MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]
                );
            }
            "reboot" => {
                crate::vga::vprintln!("Reboot requested — halting kernel (restart QEMU).");
                loop { core::hint::spin_loop(); }
            }
            "pmmstat" => {
                unsafe {
                    crate::vga::vprintln!("PMM free frames: {}", PMM.free_frames());
                    crate::vga::vprintln!("PMM total frames managed: {}", PMM.total_frames());
                }
            }
            "spawndemo" => {
                // spawn another demo task for testing concurrency
                extern "C" fn extra_demo() {
                    let mut i: u64 = 0;
                    loop {
                        crate::vga::vprintln!("extra_demo: tick {}", i);
                        i = i.wrapping_add(1);
                        for _ in 0..100_000 { core::hint::spin_loop(); }
                        crate::scheduler::task_yield();
                    }
                }
                unsafe {
                    if let Some(slot) = crate::scheduler::spawn(extra_demo, &PMM, 4) {
                        crate::vga::vprintln!("Spawned extra demo at slot {}", slot);
                    } else {
                        crate::vga::vprintln!("Failed to spawn extra demo");
                    }
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

// ---------------------- Linker-based PMM setup + fallback ------------------------

/// Linker-based PMM setup: place bitmap just after kernel image.
/// If pool too small, falls back to pmm_setup_fallback().
unsafe fn pmm_setup_linker() {
    extern "C" {
        static __kernel_start: u8;
        static __kernel_end: u8;
    }

    let kstart = &__kernel_start as *const _ as usize;
    let kend = &__kernel_end as *const _ as usize;

    // Align kernel end up to page
    let kernel_end_page = ((kend + FRAME_SIZE - 1) / FRAME_SIZE) * FRAME_SIZE;

    // Decide pool: start at kernel_end_page (min 1 MiB) up to pool_size cap
    let pool_start = if kernel_end_page < 0x0010_0000 { 0x0010_0000 } else { kernel_end_page };
    let max_pool_size = 128 * 1024 * 1024usize; // 128 MiB cap
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

    // init PMM
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
        "PMM(linker): kernel:0x{:x}-0x{:x}, pool 0x{:x}-0x{:x}, frames_managed={}",
        kstart, kend, pool_start, pool_start + pool_size, frames_managed
    );

    // quick test allocate/free
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

/// Fallback PMM setup (kept for safety): carve pool at fixed address 1 MiB, 64 MiB size
const FALLBACK_POOL_START: usize = 0x0010_0000; // 1 MiB
const FALLBACK_POOL_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

#[inline]
fn round_up_frames(bytes: usize) -> usize {
    (bytes + FRAME_SIZE - 1) / FRAME_SIZE
}

unsafe fn pmm_setup_fallback() {
    let total_frames = FALLBACK_POOL_SIZE / FRAME_SIZE;
    let bitmap_bytes_needed = (total_frames + 7) / 8;
    let bitmap_frames = round_up_frames(bitmap_bytes_needed);
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
