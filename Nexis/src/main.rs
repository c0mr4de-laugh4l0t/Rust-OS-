#![no_std]
#![no_main]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod interrupts;
mod vga;
mod kb;
mod memory;
mod scheduler;
mod task;
mod pit;
mod process;

use crate::kb::Kb;
use crate::memory::{PhysicalMemoryManager, FRAME_SIZE};
use crate::vga::VGA_WRITER;

entry_point!(kernel_main);

static mut PMM: PhysicalMemoryManager = PhysicalMemoryManager::new_uninit();

fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    interrupts::init_idt();
    interrupts::remap_pic();

    unsafe { pmm_setup_linker(); }

    Kb::init();

    pit::init(50);

    interrupts::enable_interrupts();

    {
        let mut vw = VGA_WRITER.lock();
        vw.clear_screen();
        vw.write_str("\n=== IronVeil / Nexis (Phase 3) ===\n");
        vw.write_str("Type 'help' for commands.\n\n");
    }

    extern "C" fn demo_task() {
        let mut i = 0u64;
        loop {
            crate::vga::vprintln!("[demo_task] tick {}", i);
            i = i.wrapping_add(1);
            for _ in 0..200_000 { core::hint::spin_loop(); }
            crate::scheduler::check_and_schedule();
            crate::scheduler::task_yield();
        }
    }

    extern "C" fn shell_task() {
        shell_loop();
    }

    unsafe {
        scheduler::spawn(demo_task, &PMM, 4);
        scheduler::spawn(shell_task, &PMM, 16);
    }

    loop {
        scheduler::check_and_schedule();
        x86_64::instructions::hlt();
    }
}

fn shell_loop() -> ! {
    let mut rng = crate::kb::XorShift64::new(0xabcdef123456789u64);

    loop {
        crate::vga::vprint!("ironveil@nexis:~$ ");
        crate::vga::sprint!("ironveil@nexis:~$ ");

        let line = Kb::read_line_irq();
        crate::scheduler::check_and_schedule();
        let cmd = line.trim();

        match cmd {
            "help" => {
                crate::vga::vprintln!("Available commands:");
                crate::vga::vprintln!("  help");
                crate::vga::vprintln!("  clear");
                crate::vga::vprintln!("  genpass");
                crate::vga::vprintln!("  ip");
                crate::vga::vprintln!("  mac");
                crate::vga::vprintln!("  reboot");
            }
            "clear" => VGA_WRITER.lock().clear_screen(),
            "genpass" => {
                let mut pass = [0u8; 16];
                for i in 0..16 {
                    pass[i] = rng.next_range_u8(33, 126);
                }
                let p = unsafe { core::str::from_utf8_unchecked(&pass) };
                crate::vga::vprintln!("Password: {}", p);
            }
            "ip" => {
                crate::vga::vprintln!(
                    "Fake IPv4: {}.{}.{}.{}",
                    rng.next_range_u8(10, 250),
                    rng.next_range_u8(1, 254),
                    rng.next_range_u8(1, 254),
                    rng.next_range_u8(1, 254)
                );
            }
            "mac" => {
                let mut parts = [0u8; 6];
                for i in 0..6 {
                    parts[i] = rng.next_u8();
                }
                crate::vga::vprintln!(
                    "Fake MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]
                );
            }
            "reboot" => loop { core::hint::spin_loop(); },
            "" => {}
            _ => crate::vga::vprintln!("Unknown command: '{}'", cmd),
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::vga::vprintln!("\n*** PANIC ***");
    if let Some(loc) = info.location() {
        crate::vga::vprintln!("At {}:{}: {}", loc.file(), loc.line(), info);
    } else {
        crate::vga::vprintln!("{}", info);
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

const FALLBACK_POOL_START: usize = 0x0010_0000;
const FALLBACK_POOL_SIZE: usize = 64 * 1024 * 1024;

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