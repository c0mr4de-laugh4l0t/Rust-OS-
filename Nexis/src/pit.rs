use x86_64::instructions::port::Port;
use core::sync::atomic::{AtomicU64, Ordering};

static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

pub fn init(hz: u32) {
    let divisor = if hz == 0 { 0 } else { (1193182u32 / hz) as u16 };
    unsafe {
        let mut cmd = Port::<u8>::new(0x43);
        let mut data = Port::<u8>::new(0x40);
        cmd.write(0x36);
        data.write((divisor & 0xFF) as u8);
        data.write(((divisor >> 8) & 0xFF) as u8);
    }
}

pub fn tick() -> u64 {
    TICK_COUNT.fetch_add(1, Ordering::SeqCst) + 1
}

pub fn ticks() -> u64 {
    TICK_COUNT.load(Ordering::SeqCst)
}