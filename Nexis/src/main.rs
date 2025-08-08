#![no_std]
#![no_main]

// Nexis kernel: VGA + PS/2 keyboard shell demo (on-screen)
// Works in QEMU. Use `-serial stdio` if you still want serial.

use core::panic::PanicInfo;
use core::fmt::Write;
use bootloader::{entry_point, BootInfo};
use uart_16550::SerialPort;
use spin::Mutex;
use lazy_static::lazy_static;

// keyboard / port access
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use x86_64::instructions::port::Port;

// VGA constants & types
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;
const VGA_BUFFER_ADDR: usize = 0xb8000;

entry_point!(kernel_main);

lazy_static! {
    static ref SERIAL1: Mutex<SerialPort> = {
        let mut sp = unsafe { SerialPort::new(0x3F8) };
        sp.init();
        Mutex::new(sp)
    };
    static ref VGA_WRITER: Mutex<VgaWriter> = Mutex::new(VgaWriter::new());
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

/// Simple VGA text-mode writer
struct VgaWriter {
    column: usize,
    row: usize,
    color: u8,
    buffer: *mut u8,
}
impl VgaWriter {
    const fn new() -> Self {
        Self {
            column: 0,
            row: 0,
            color: 0x0f, // white on black
            buffer: VGA_BUFFER_ADDR as *mut u8,
        }
    }

    fn put_char(&mut self, c: char) {
        match c {
            '\n' => {
                self.new_line();
                return;
            }
            '\r' => { self.column = 0; return; }
            _ => {}
        }
        if self.column >= BUFFER_WIDTH {
            self.new_line();
        }
        let offset = (self.row * BUFFER_WIDTH + self.column) * 2;
        unsafe {
            let char_byte = c as u8;
            core::ptr::write_volatile(self.buffer.add(offset), char_byte);
            core::ptr::write_volatile(self.buffer.add(offset + 1), self.color);
        }
        self.column += 1;
    }

    fn write_str(&mut self, s: &str) {
        for c in s.chars() {
            self.put_char(c);
        }
    }

    fn new_line(&mut self) {
        self.column = 0;
        if self.row + 1 < BUFFER_HEIGHT {
            self.row += 1;
        } else {
            // scroll up by one line
            for r in 1..BUFFER_HEIGHT {
                for col in 0..BUFFER_WIDTH {
                    let src = ((r * BUFFER_WIDTH) + col) * 2;
                    let dst = (((r - 1) * BUFFER_WIDTH) + col) * 2;
                    unsafe {
                        let ch = core::ptr::read_volatile(self.buffer.add(src));
                        let color = core::ptr::read_volatile(self.buffer.add(src + 1));
                        core::ptr::write_volatile(self.buffer.add(dst), ch);
                        core::ptr::write_volatile(self.buffer.add(dst + 1), color);
                    }
                }
            }
            // clear last line
            let last = (BUFFER_HEIGHT - 1) * BUFFER_WIDTH * 2;
            for col in 0..BUFFER_WIDTH {
                unsafe {
                    core::ptr::write_volatile(self.buffer.add(last + col * 2), b' ');
                    core::ptr::write_volatile(self.buffer.add(last + col * 2 + 1), self.color);
                }
            }
        }
    }

    fn clear_screen(&mut self) {
        for r in 0..BUFFER_HEIGHT {
            for c in 0..BUFFER_WIDTH {
                let offset = (r * BUFFER_WIDTH + c) * 2;
                unsafe {
                    core::ptr::write_volatile(self.buffer.add(offset), b' ');
                    core::ptr::write_volatile(self.buffer.add(offset + 1), self.color);
                }
            }
        }
        self.row = 0;
        self.column = 0;
    }
}

impl core::fmt::Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

// ----- PS/2 reading -----
fn read_scancode() -> u8 {
    // Port 0x60 has data register
    let mut port = Port::<u8>::new(0x60);
    unsafe { port.read() }
}

// Non-blocking poll for scancode: returns 0 if none
fn poll_scancode() -> u8 {
    // To check if data available, read status port 0x64 bit 0
    let mut status = Port::<u8>::new(0x64);
    let s = unsafe { status.read() };
    if s & 0x01 == 0x01 {
        // data ready
        read_scancode()
    } else {
        0
    }
}

// simple sleep
fn small_delay() {
    for _ in 0..1000 { core::hint::spin_loop(); }
}

// ----- kernel main & shell -----
fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    {
        let mut v = VGA_WRITER.lock();
        v.clear_screen();
        v.write_str("\n=== IronVeil / Nexis (VGA) ===\n");
        v.write_str("On-screen console ready. Type 'help'.\n\n");
    }
    // also print serial banner
    sprintln!("\n=== IronVeil / Nexis (serial) ===");
    sprintln!("On-screen console ready. Type 'help' and press Enter.\n");

    // keyboard state
    let mut keyboard: Keyboard<layouts::Us104Key, ScancodeSet1> =
        Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore);

    serial_vga_shell_loop(&mut keyboard)
}

fn serial_vga_shell_loop(keyboard: &mut Keyboard<layouts::Us104Key, ScancodeSet1>) -> ! {
    let mut line_buf = [0u8; 256];
    let mut len = 0usize;

    loop {
        // prompt
        {
            let mut v = VGA_WRITER.lock();
            v.write_str("ironveil@nexis:~$ ");
        }
        sprint!("ironveil@nexis:~$ ");

        // read characters until newline
        len = 0;
        loop {
            small_delay();
            let sc = poll_scancode();
            if sc != 0 {
                if let Ok(Some(key_event)) = keyboard.add_byte(sc) {
                    if let Some(key) = keyboard.process_keyevent(key_event) {
                        match key {
                            DecodedKey::Unicode(ch) => {
                                // print char
                                let c = ch;
                                {
                                    let mut v = VGA_WRITER.lock();
                                    v.put_char(c);
                                }
                                sprint!("{}", c);
                                if c == '\n' || c == '\r' {
                                    break;
                                } else if c == '\x08' {
                                    if len > 0 { len -= 1; }
                                } else {
                                    if len < line_buf.len() - 1 {
                                        line_buf[len] = c as u8;
                                        len += 1;
                                    }
                                }
                            }
                            DecodedKey::RawKey(key) => {
                                // handle Enter as RawKey::Enter
                                if format!("{:?}", key) == "Enter" {
                                    // print newline
                                    {
                                        let mut v = VGA_WRITER.lock();
                                        v.put_char('\n');
                                    }
                                    sprintln!("");
                                    break;
                                } else if format!("{:?}", key) == "Backspace" {
                                    // backspace handling
                                    if len > 0 { len -= 1; }
                                    {
                                        let mut v = VGA_WRITER.lock();
                                        // move back, replace with space, move back
                                        v.put_char('\x08'); v.put_char(' '); v.put_char('\x08');
                                    }
                                    sprint!("\x08 \x08");
                                }
                            }
                        }
                    }
                }
            }
            // Also check serial input (optional)
            // if you want to accept serial input, implement serial_read_byte() etc.
        }

        // convert buffer to &str
        let cmd = unsafe { core::str::from_utf8_unchecked(&line_buf[..len]) }.trim();

        // execute command and print to both VGA and serial
        match cmd {
            "help" => {
                vprintln!("Available commands:");
                vprintln!("  help       - this message");
                vprintln!("  clear|cls  - clear screen");
                vprintln!("  genpass    - generate 16-char password");
                vprintln!("  ip         - fake IPv4");
                vprintln!("  mac        - fake MAC");
                vprintln!("  reboot     - halt (use QEMU restart)");
            }
            "clear" | "cls" => {
                VGA_WRITER.lock().clear_screen();
            }
            "genpass" => {
                let mut p = [0u8; 16];
                // simple xorshift
                let mut s = 0x123456789abcdefu64;
                for i in 0..16 {
                    s ^= s << 13;
                    s ^= s >> 7;
                    s ^= s << 17;
                    p[i] = (s & 0x7F) as u8;
                    if p[i] < 33 { p[i] = 33 + (p[i] % 94); }
                }
                let pass = unsafe { core::str::from_utf8_unchecked(&p) };
                vprintln!("Generated password: {}", pass);
            }
            "ip" => {
                let mut s = 0xabcdef12345u64;
                s ^= s << 13; s ^= s >> 7; s ^= s << 17;
                let a = (s & 0xFF) % 240 + 10;
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                let b = (s & 0xFF) % 254 + 1;
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                let c = (s & 0xFF) % 254 + 1;
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                let d = (s & 0xFF) % 254 + 1;
                vprintln!("Fake IPv4: {}.{}.{}.{}", a, b, c, d);
            }
            "mac" => {
                let mut s = 0xdeadbeefu64;
                let mut parts = [0u8; 6];
                for i in 0..6 {
                    s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                    parts[i] = (s & 0xFF) as u8;
                }
                vprintln!("Fake MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]);
            }
            "reboot" => {
                vprintln!("Reboot requested — halting kernel (restart QEMU).");
                loop { core::hint::spin_loop(); }
            }
            "" => {}
            _ => {
                vprintln!("Unknown command: '{}'. Type 'help'.", cmd);
            }
        }

        // clear input buffer
        for i in 0..len { line_buf[i] = 0; }
        len = 0;
    }
}

// helper to print to VGA + serial
fn vprintln_impl(args: core::fmt::Arguments) {
    // VGA
    {
        let mut v = VGA_WRITER.lock();
        let _ = v.write_fmt(args);
        v.put_char('\n');
    }
    // serial
    serial_print(args);
    serial_print(format_args!("\n"));
}
macro_rules! vprintln {
    ($($arg:tt)*) => ($crate::vprintln_impl(format_args!($($arg)*)));
}
macro_rules! vprint {
    ($($arg:tt)*) => ({
        let mut v = VGA_WRITER.lock();
        let _ = v.write_fmt(format_args!($($arg)*));
        let _ = serial_print(format_args!($($arg)*));
    });
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vprintln!("\n\n*** PANIC ***");
    if let Some(loc) = info.location() {
        vprintln!("panic at {}:{}: {}", loc.file(), loc.line(), info);
    } else {
        vprintln!("panic: {}", info);
    }
    loop { core::hint::spin_loop(); }
}
