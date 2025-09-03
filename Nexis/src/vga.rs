use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;
use core::fmt::Write;

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;
const VGA_BUFFER_ADDR: usize = 0xb8000;

lazy_static! {
    pub static ref VGA_WRITER: Mutex<VgaWriter> = Mutex::new(VgaWriter::new());
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut sp = unsafe { SerialPort::new(0x3F8) };
        sp.init();
        Mutex::new(sp)
    };
}

// ---- Serial printing ----
pub fn serial_print(args: core::fmt::Arguments) {
    let mut s = SERIAL1.lock();
    let _ = s.write_fmt(args);
}
pub fn serial_println(args: core::fmt::Arguments) {
    serial_print(args);
    serial_print(format_args!("\n"));
}

macro_rules! sprintln {
    ($($arg:tt)*) => (crate::vga::serial_println(format_args!($($arg)*)));
}
macro_rules! sprint {
    ($($arg:tt)*) => (crate::vga::serial_print(format_args!($($arg)*)));
}
pub(crate) use sprintln;
pub(crate) use sprint;

// ---- VGA Writer ----
pub struct VgaWriter {
    column: usize,
    row: usize,
    color: u8,
    buffer: *mut u8,
}

// Safety: VgaWriter writes directly to a memory-mapped buffer.
// We guarantee only one global instance, so it's safe.
unsafe impl Send for VgaWriter {}
unsafe impl Sync for VgaWriter {}

impl VgaWriter {
    pub const fn new() -> Self {
        Self {
            column: 0,
            row: 0,
            color: 0x0f,
            buffer: VGA_BUFFER_ADDR as *mut u8,
        }
    }

    pub fn put_char(&mut self, c: char) {
        match c {
            '\n' => { self.new_line(); return; }
            '\r' => { self.column = 0; return; }
            _ => {}
        }
        if self.column >= BUFFER_WIDTH { self.new_line(); }
        let offset = (self.row * BUFFER_WIDTH + self.column) * 2;
        unsafe {
            core::ptr::write_volatile(self.buffer.add(offset), c as u8);
            core::ptr::write_volatile(self.buffer.add(offset + 1), self.color);
        }
        self.column += 1;
    }

    pub fn write_str(&mut self, s: &str) {
        for c in s.chars() { self.put_char(c); }
    }

    pub fn new_line(&mut self) {
        self.column = 0;
        if self.row + 1 < BUFFER_HEIGHT {
            self.row += 1;
        } else {
            // scroll up
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

    pub fn clear_screen(&mut self) {
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

// ---- VGA + Serial printing ----
pub fn vprintln_impl(args: core::fmt::Arguments) {
    {
        let mut v = VGA_WRITER.lock();
        let _ = v.write_fmt(args);
        v.put_char('\n');
    }
    serial_print(args);
    serial_print(format_args!("\n"));
}

macro_rules! vprintln {
    ($($arg:tt)*) => (crate::vga::vprintln_impl(format_args!($($arg)*)));
}
macro_rules! vprint {
    ($($arg:tt)*) => ({
        let mut v = VGA_WRITER.lock();
        let _ = v.write_fmt(format_args!($($arg)*));
        let _ = crate::vga::serial_print(format_args!($($arg)*));
    });
}
pub(crate) use vprintln;
pub(crate) use vprint;
