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
    interrupts::enable_interrupts();

    {
        let mut vw = VGA_WRITER.lock();
        vw.clear_screen();
        vw.write_str("\n=== IronVeil / Nexis (Phase 3) ===\n");
        vw.write_str("Type 'help' for commands.\n\n");
    }

    unsafe { pmm_setup_linker(); }

    Kb::init();

    pit::init(100);

    extern "C" fn demo_task() {
        let mut i = 0u64;
        loop {
            crate::vga::vprintln!("[demo_task] tick {}", i);
            i = i.wrapping_add(1);
            for _ in 0..500_000 { core::hint::spin_loop(); }
            scheduler::task_yield();
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
    let pool_size = 64 * 1024 * 1024;
    PMM.init(pool_start as *mut u8, pool_size, pool_start / FRAME_SIZE, pool_size / FRAME_SIZE);
}