#![no_std]
#![no_main]

// Nexis: IRQ keyboard (low-level PIC + IDT) + VGA shell
// Keep your existing commands and behavior; input now arrives via IRQ.

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod interrupts;
mod vga;
mod kb;

use vga::VgaWriter;
use crate::vga::VGA_WRITER;
use crate::kb::Kb;

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    // init IDT + PIC
    interrupts::init_idt();
    interrupts::remap_pic();
    interrupts::enable_interrupts();

    // init VGA & clear
    {
        let mut vw = VGA_WRITER.lock();
        vw.clear_screen();
        vw.write_str("\n=== IronVeil / Nexis (VGA IRQ keyboard) ===\n");
        vw.write_str("On-screen console ready. Type 'help'.\n\n");
    }

    // Print to serial too (optional)
    crate::vga::sprintln!("\n=== IronVeil / Nexis (serial) ===");
    crate::vga::sprintln!("IRQ keyboard active. Type 'help' and press Enter.\n");

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
