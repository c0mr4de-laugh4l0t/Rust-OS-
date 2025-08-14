#![no_std]
#![no_main]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod alloc;
mod memory;
mod process;
mod task;
mod scheduler;
mod context;

mod interrupts;
mod pit;
mod kb;
mod vga;

mod syscall;
mod syscall_dispatch;

mod fs;
mod userland;

use vga::VGA_WRITER;
use crate::kb::Kb;
use memory::PhysicalMemoryManager;

entry_point!(kernel_main);

static mut PMM: PhysicalMemoryManager = PhysicalMemoryManager::new_uninit();

fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    interrupts::init_idt();
    interrupts::remap_pic();
    interrupts::enable_interrupts();

    VGA_WRITER.lock().clear_screen();
    VGA_WRITER.lock().write_str("\n=== IronVeil / Nexis Kernel ===\n");

    unsafe {
        memory::pmm_setup_linker();
    }

    fs::fs_init();
    Kb::init();

    extern "C" fn shell_task() {
        shell_loop();
    }

    unsafe {
        scheduler::spawn(shell_task, &PMM, 16);
    }

    pit::init(100);
    scheduler::schedule_loop();
}

fn shell_loop() -> ! {
    let mut rng = kb::XorShift64::new(0xabcdef123456789u64);

    loop {
        vga::vprint!("ironveil@nexis:~$ ");
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
                vga::vprintln!("  reboot     - halt");
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
                vga::vprintln!(
                    "Fake MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]
                );
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
    vga::vprintln!("KERNEL PANIC: {}", info);
    loop {}
}