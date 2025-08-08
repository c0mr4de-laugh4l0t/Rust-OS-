#![no_std]
#![no_main]

// Nexis kernel: serial shell, usable demo kernel.

use core::panic::PanicInfo;
use core::fmt::Write;
use bootloader::{entry_point, BootInfo};
use uart_16550::SerialPort;
use spin::Mutex;
use lazy_static::lazy_static;

entry_point!(kernel_main);

lazy_static! {
    static ref SERIAL1: Mutex<SerialPort> = {
        // Standard QEMU serial port I/O port 0x3f8
        let mut sp = unsafe { SerialPort::new(0x3F8) };
        sp.init();
        Mutex::new(sp)
    };
}

fn serial_print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    let mut s = SERIAL1.lock();
    let _ = s.write_fmt(args);
}

macro_rules! sprintln {
    ($($arg:tt)*) => ($crate::serial_print(format_args!($($arg)*)));
}
macro_rules! sprint {
    ($($arg:tt)*) => ($crate::serial_print(format_args!($($arg)*)));
}

fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    // Banner
    sprintln!("\n\n=== IronVeil / Nexis Kernel ===\n");
    sprintln!("Serial console ready. Type 'help' and press Enter.\n");

    serial_shell_loop();
}

/// Simple deterministic RNG (xorshift64*) - small and no_std
struct XorShift64 {
    state: u64,
}
impl XorShift64 {
    fn new(seed: u64) -> Self {
        let mut s = seed;
        if s == 0 { s = 0x123456789abcdefu64; }
        Self { state: s }
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
    fn next_u8(&mut self) -> u8 { (self.next_u64() & 0xFF) as u8 }
    fn next_range_u8(&mut self, low: u8, high: u8) -> u8 {
        let r = self.next_u8();
        low + (r % (high - low + 1))
    }
}

/// Read/write buffer helpers over serial
fn serial_read_line(buf: &mut [u8]) -> usize {
    let mut i = 0usize;
    loop {
        let c = serial_read_byte();
        match c {
            b'\r' => { /* ignore */ }
            b'\n' => {
                sprint!("\n");
                break;
            }
            8 | 127 => { // backspace
                if i > 0 {
                    i -= 1;
                    sprint!("\x08 \x08"); // move back, space, move back
                }
            }
            b => {
                if i < buf.len() - 1 && b >= 32 && b < 127 {
                    buf[i] = b;
                    i += 1;
                    let ch = b as char;
                    sprint!("{}", ch);
                }
            }
        }
    }
    buf[i] = 0;
    i
}

fn serial_read_byte() -> u8 {
    loop {
        let b = {
            let mut s = SERIAL1.lock();
            // read if data available
            if s.is_transmit_empty() {
                // nothing waiting — but uart_16550 crate doesn't expose available check,
                // instead we poll the port status via reading the line status register:
                // uart_16550::SerialPort provides read() that blocks? it reads port and returns u8.
            }
            // We'll use unsafe read to poll: using SerialPort.read gives a u8 directly (nonblocking),
            // but the crate read() returns u8; it doesn't block.
            // So we call read() in a loop.
            s.read()
        };
        if b != 0 { return b; }
        // small busy wait
        for _ in 0..1000 { core::hint::spin_loop(); }
    }
}

fn serial_shell_loop() -> ! {
    let mut rng = XorShift64::new(0xabcdef123456789u64);
    let mut line_buf = [0u8; 256];

    loop {
        sprint!("ironveil@nexis:~$ ");
        let len = serial_read_line(&mut line_buf);
        if len == 0 {
            continue;
        }
        // parse command
        // convert to lowercase & str
        let cmd = {
            // safe to create &str because we nul-terminate
            let s = unsafe { core::str::from_utf8_unchecked(&line_buf[..len]) };
            s.trim()
        };

        match cmd {
            "help" => {
                sprintln!("Available commands:");
                sprintln!("  help       - this message");
                sprintln!("  cls|clear  - clear the serial console (simulated)");
                sprintln!("  genpass    - generate a 16-char password");
                sprintln!("  ip         - fake IPv4 address");
                sprintln!("  mac        - fake MAC address");
                sprintln!("  reboot     - halt (in QEMU use machine restart)");
            }
            "cls" | "clear" => {
                // can't really clear terminal on remote; print many newlines
                for _ in 0..30 { sprintln!(""); }
            }
            "genpass" => {
                let mut pass = [0u8; 16];
                for i in 0..16 {
                    let b = rng.next_range_u8(33u8, 126u8);
                    pass[i] = b;
                }
                let p = unsafe { core::str::from_utf8_unchecked(&pass) };
                sprintln!("Generated password: {}", p);
            }
            "ip" => {
                let a = rng.next_range_u8(10, 250);
                let b = rng.next_range_u8(1, 254);
                let c = rng.next_range_u8(1, 254);
                let d = rng.next_range_u8(1, 254);
                sprintln!("Fake IPv4: {}.{}.{}.{}", a, b, c, d);
            }
            "mac" => {
                let mut parts = [0u8; 6];
                for i in 0..6 {
                    parts[i] = rng.next_u8();
                }
                sprintln!("Fake MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]);
            }
            "reboot" => {
                sprintln!("Reboot requested — halting (use QEMU restart).");
                // halt by infinite loop
                loop { core::hint::spin_loop(); }
            }
            "" => {}
            _ => {
                sprintln!("Unknown command: '{}'. Type 'help'.", cmd);
            }
        }
    }
}

/// Panic handler prints to serial and halts
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    sprintln!("\n\n*** PANIC ***");
    if let Some(loc) = info.location() {
        sprintln!("panic at {}:{}: {}", loc.file(), loc.line(), info);
    } else {
        sprintln!("panic: {}", info);
    }
    loop { core::hint::spin_loop(); }
        }
