#![no_std]
#![no_main]

pub mod alloc;
pub mod context;
pub mod interrupt;
pub mod pit;
pub mod kb;
pub mod vga;
pub mod memory;
pub mod task;
pub mod scheduler;
pub mod process;
pub mod syscall;
pub mod syscall_dispatch;
pub mod fs;
pub mod userland;
pub mod lib;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use vga::VgaWriter;
use crate::vga::VGA_WRITER;
use crate::kb::Kb;
use memory::PhysicalMemoryManager;

static mut PMM: PhysicalMemoryManager = PhysicalMemoryManager::new_uninit();

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    interrupt::init_idt();
    interrupt::remap_pic();
    interrupt::enable_interrupts();

    {
        let mut vw = VGA_WRITER.lock();
        vw.clear_screen();
        vw.write_str("\n=== IronVeil / Nexis Kernel ===\n");
        vw.write_str("Type 'help' for commands.\n\n");
    }

    crate::vga::sprintln!("\n=== IronVeil / Nexis Kernel (serial) ===");

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

    scheduler::schedule_loop()
}

fn shell_loop() -> ! {
    let mut rng = kb::XorShift64::new(0xabcdef123456789u64);

    loop {
        vga::vprint!("ironveil@nexis:~$ ");
        vga::sprint!("ironveil@nexis:~$ ");

        let line = Kb::read_line_irq();
        let cmd = line.trim();

        match cmd {
            "help" => {
                vga::vprintln!("Available commands:");
                vga::vprintln!("  help       - this message");
                vga::vprintln!("  clear|cls  - clear screen");
                vga::vprintln!("  genpass    - generate a 16-char password");
                vga::vprintln!("  ip         - fake IPv4");
                vga::vprintln!("  mac        - fake MAC");
                vga::vprintln!("  reboot     - halt (restart QEMU)");
                vga::vprintln!("  fs ls      - list files");
                vga::vprintln!("  fs cat <file> - print file contents");
            }
            "clear" | "cls" => {
                VGA_WRITER.lock().clear_screen();
            }
            "genpass" => {
                let mut pass = [0u8; 16];
                for i in 0..16 {
                    pass[i] = rng.next_range_u8(33u8, 126u8);
                }
                let p = unsafe { core::str::from_utf8_unchecked(&pass) };
                vga::vprintln!("Generated password: {}", p);
            }
            "ip" => {
                let a = rng.next_range_u8(10, 250);
                let b = rng.next_range_u8(1, 254);
                let c = rng.next_range_u8(1, 254);
                let d = rng.next_range_u8(1, 254);
                vga::vprintln!("Fake IPv4: {}.{}.{}.{}", a, b, c, d);
            }
            "mac" => {
                let mut parts = [0u8; 6];
                for i in 0..6 { parts[i] = rng.next_u8(); }
                vga::vprintln!("Fake MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]);
            }
            "reboot" => {
                vga::vprintln!("Reboot requested — halting kernel.");
                loop { core::hint::spin_loop(); }
            }
            x if x.starts_with("fs ls") => {
                fs::list_files();
            }
            x if x.starts_with("fs cat ") => {
                let parts: Vec<&str> = x.splitn(3, ' ').collect();
                if parts.len() == 3 {
                    fs::print_file(parts[2]);
                } else {
                    vga::vprintln!("Usage: fs cat <filename>");
                }
            }
            "" => {}
            _ => vga::vprintln!("Unknown command: '{}'. Type 'help'.", cmd),
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga::vprintln!("\n\n*** PANIC ***");
    if let Some(loc) = info.location() {
        vga::vprintln!("panic at {}:{}: {}", loc.file(), loc.line(), info);
    } else {
        vga::vprintln!("panic: {}", info);
    }
    loop { core::hint::spin_loop(); }
}