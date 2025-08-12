use x86_64::instructions::port::Port;
use core::sync::atomic::{AtomicBool, Ordering};

pub static NEED_RESCHED: AtomicBool = AtomicBool::new(false);

pub fn init(hz: u32) {
    let divisor = 1193180 / hz;
    unsafe {
        let mut cmd = Port::<u8>::new(0x43);
        let mut data = Port::<u8>::new(0x40);
        cmd.write(0x36);
        data.write((divisor & 0xFF) as u8);
        data.write(((divisor >> 8) & 0xFF) as u8);
    }
}

pub extern "x86-interrupt" fn pit_handler(_stack_frame: &mut x86_64::structures::idt::InterruptStackFrame) {
    NEED_RESCHED.store(true, Ordering::SeqCst);
    unsafe {
        let mut cmd = Port::<u8>::new(0x20);
        cmd.write(0x20);
    }
}