#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod interrupts;
mod vga;
mod kb;
mod memory;
mod scheduler;
mod alloc;
mod fs;

use vga::VgaWriter;
use crate::vga::VGA_WRITER;
use crate::kb::Kb;
use memory::PhysicalMemoryManager;

entry_point!(kernel_main);

static mut PMM: PhysicalMemoryManager = PhysicalMemoryManager::new_uninit();

fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    interrupts::init_idt();
    interrupts::remap_pic();
    interrupts::enable_interrupts();

    {
        let mut vw = VGA_WRITER.lock();
        vw.clear_screen();
        vw.write_str("\n=== IronVeil / Nexis Kernel ===\n");
        vw.write_str("Type 'help' for commands.\n\n");
    }

    unsafe {
        pmm_setup_linker();
        crate::alloc::init_heap(&PMM);
    }

    crate::fs::fs_init();
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
                crate::vga::vprintln!("Reboot requested — halting kernel.");
                loop { core::hint::spin_loop(); }
            }
            x if x.starts_with("fs ls") => {
                crate::fs::list_files();
            }
            x if x.starts_with("fs cat ") => {
                let parts: alloc::vec::Vec<&str> = x.splitn(3, ' ').collect();
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
    crate::vga::vprintln!("KERNEL PANIC: {}", info);
    loop { core::hint::spin_loop(); }
}