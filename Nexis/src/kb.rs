use spin::Mutex;
use lazy_static::lazy_static;
use pc_keyboard::{Keyboard, layouts, ScancodeSet1, DecodedKey, HandleControl};
use x86_64::instructions::hlt;
use core::str;

pub struct XorShift64 { state: u64 }
impl XorShift64 {
    pub fn new(seed: u64) -> Self { Self { state: seed } }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
    pub fn next_u8(&mut self) -> u8 { (self.next_u64() & 0xFF) as u8 }
    pub fn next_range_u8(&mut self, low: u8, high: u8) -> u8 {
        let r = self.next_u8();
        low + (r % (high - low + 1))
    }
}

const BUF_SIZE: usize = 1024;

lazy_static! {
    static ref SCANCODE_QUEUE: Mutex<ScancodeQueue> = Mutex::new(ScancodeQueue::new());
}

struct ScancodeQueue {
    buf: [u8; BUF_SIZE],
    head: usize,
    tail: usize,
}
impl ScancodeQueue {
    const fn new() -> Self {
        Self { buf: [0; BUF_SIZE], head: 0, tail: 0 }
    }
    fn push(&mut self, sc: u8) {
        let next = (self.head + 1) % BUF_SIZE;
        if next != self.tail {
            self.buf[self.head] = sc;
            self.head = next;
        }
        // else drop if buffer full
    }
    fn pop(&mut self) -> Option<u8> {
        if self.tail == self.head { return None; }
        let sc = self.buf[self.tail];
        self.tail = (self.tail + 1) % BUF_SIZE;
        Some(sc)
    }
}

pub struct Kb;
impl Kb {
    pub fn init() {
        // nothing for now; PIC & IDT set up elsewhere
    }
    pub fn push_scancode(sc: u8) {
        SCANCODE_QUEUE.lock().push(sc);
    }
    /// blocking read of a single scancode; sleeps with `hlt` if none
    fn read_scancode_blocking() -> u8 {
        loop {
            if let Some(sc) = SCANCODE_QUEUE.lock().pop() {
                return sc;
            }
            hlt(); // wait for next interrupt
        }
    }
    /// read bytes until newline, return owned String (heapless using stack buffer and return &str)
    pub fn read_line_irq() -> &'static str {
        // static buffer to store an owned line; we keep it static for simplicity
        static mut LINE_BUF: [u8; 256] = [0; 256];
        let mut len = 0usize;
        let mut keyboard: Keyboard<layouts::Us104Key, ScancodeSet1> =
            Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore);

        loop {
            let sc = Self::read_scancode_blocking();
            if let Ok(Some(event)) = keyboard.add_byte(sc) {
                if let Some(key) = keyboard.process_keyevent(event) {
                    match key {
                        DecodedKey::Unicode(ch) => {
                            // echo to VGA/serial
                            crate::vga::VGA_WRITER.lock().put_char(ch);
                            crate::vga::sprint!("{}", ch);
                            if ch == '\r' || ch == '\n' {
                                crate::vga::vprintln!("");
                                break;
                            } else if ch == '\x08' {
                                if len > 0 { len -= 1; }
                            } else {
                                if len < 255 {
                                    unsafe { LINE_BUF[len] = ch as u8; }
                                    len += 1;
                                }
                            }
                        }
                        DecodedKey::RawKey(k) => {
                            // Enter and Backspace can appear as raw keys
                            if format!("{:?}", k) == "Enter" {
                                crate::vga::vprintln!("");
                                break;
                            } else if format!("{:?}", k) == "Backspace" {
                                if len > 0 { len -= 1; }
                                crate::vga::sprint!("\x08 \x08");
                            }
                        }
                    }
                }
            }
        }
        unsafe {
            LINE_BUF[len] = 0;
            let s = core::str::from_utf8_unchecked(&LINE_BUF[..len]);
            // leak a &'static str: acceptable for small demo
            // allocate a static arena? for demo, returning a static slice pointing to LINE_BUF is fine.
            s
        }
    }
  }
