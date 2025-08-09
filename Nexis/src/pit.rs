// pit.rs
#![no_std]

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

const PIT_CHANNEL0: u16 = 0x40;
const PIT_COMMAND: u16  = 0x43;
const PIT_FREQ: u32     = 1_193_182; // Hz base

static TICK_COUNT: AtomicU64 = AtomicU64::new(0);
pub static NEED_RESCHED: AtomicBool = AtomicBool::new(false);

pub fn init(hz: u32) {
    if hz == 0 { return; }
    let divisor: u16 = (PIT_FREQ / hz) as u16;
    unsafe {
        outb(PIT_COMMAND, 0x34); // channel 0, lobyte/hibyte, mode 2
        outb(PIT_CHANNEL0, (divisor & 0xFF) as u8);
        outb(PIT_CHANNEL0, (divisor >> 8) as u8);
    }
}

pub fn tick() -> u64 {
    TICK_COUNT.fetch_add(1, Ordering::SeqCst) + 1
}

pub fn ticks() -> u64 {
    TICK_COUNT.load(Ordering::SeqCst)
}

#[inline(always)]
unsafe fn outb(port: u16, val: u8) {
    use core::arch::asm;
    asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags));
  }
