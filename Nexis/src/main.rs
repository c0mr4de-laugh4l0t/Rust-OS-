#![no_std]
#![no_main]

// Nexis kernel: serial shell demo (usable in QEMU with -serial stdio)

use core::panic::PanicInfo;
use core::fmt::Write;
use bootloader::{entry_point, BootInfo};
use uart_16550::SerialPort;
use spin::Mutex;
use lazy_static::lazy_static;

entry_point!(kernel_main);

lazy_static! {
    static ref SERIAL1: Mutex<SerialPort> = {
        // QEMU standard serial I/O port 0x3F8
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
    sprintln!("\n\n=== IronVeil / Nexis Kernel ===");
    sprintln!("Serial console ready. Type 'help' and press Enter.\n");
    serial_shell_loop();
}

/// Small xorshift RNG (no_std)
struct XorShift64 { state: u64 }
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
        // avoid zero division danger; assume high >= low
        low + (r % (high - low + 1))
    }
}

fn serial_read_byte() -> u8 {
    loop {
        let b = {
            let mut s = SERIAL1.lock();
            // SerialPort::read returns u8, 0 if no data (crate behavior)
            s.read()
        };
        if b != 0 {
            return b;
        }
        // tiny pause
        for _ in 0..1000 { core::hint::spin_loop(); }
    }
}

fn serial_read_line(buf: &mut [u8]) -> usize {
    let mut i = 0usize;
    loop {
        let c = serial_read_byte();
        match c {
            b'\r' => {},
            b'\n' => { sprint!("\n"); break; }
            8 | 127 => {
                if i > 0 {
                    i -= 1;
                    sprint!("\x08 \x08");
                }
            }
            b if b >= 32 && b < 127 => {
                if i < buf.len() - 1 {
                    buf[i] = b;
                    i += 1;
                    sprint!("{}", b as char);
                }
            }
            _ => {}
        }
    }
    buf[i] = 0;
    i
}

fn serial_shell_loop() -> ! {
    let mut rng = XorShift64::new(0xabcdef123456789u64);
    let mut line_buf = [0u8; 256];

    loop {
        sprint!("ironveil@nexis:~$ ");
        let len = serial_read_line(&mut line_buf);
        if len == 0 { continue; }

        let cmd = unsafe { core::str::from_utf8_unchecked(&line_buf[..len]) }.trim();

        match cmd {
            "help" => {
                sprintln!("Available commands:");
                sprintln!("  help       - this message");
                sprintln!("  cls|clear  - clear the console output");
                sprintln!("  genpass    - generate a 16-char password");
                sprintln!("  ip         - fake IPv4 address");
                sprintln!("  mac        - fake MAC address");
                sprintln!("  reboot     - halt (use QEMU restart)");
            }
            "cls" | "clear" => {
                for _ in 0..40 { sprintln!(""); }
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
                for i in 0..6 { parts[i] = rng.next_u8(); }
                sprintln!("Fake MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]);
            }
            "reboot" => {
                sprintln!("Reboot requested — halting kernel (use QEMU restart).");
                loop { core::hint::spin_loop(); }
            }
            "" => {}
            _ => {
                sprintln!("Unknown command: '{}'. Type 'help'.", cmd);
            }
        }
    }
}

/// Panic handler prints panic info to serial then halts
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
