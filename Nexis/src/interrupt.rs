use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::{interrupts, port::Port};

use crate::kb::Kb;

pub const PIC1_COMMAND: u16 = 0x20;
pub const PIC1_DATA: u16 = 0x21;
pub const PIC2_COMMAND: u16 = 0xA0;
pub const PIC2_DATA: u16 = 0xA1;

lazy_static! {
    static ref IDT: Mutex<Option<InterruptDescriptorTable>> = Mutex::new(None);
}

pub fn init_idt() {
    use x86_64::structures::idt::InterruptDescriptorTable;
    let mut idt = InterruptDescriptorTable::new();
    // keyboard IRQ is mapped to vector 0x21 (33) after PIC remap
    idt[33].set_handler_fn(keyboard_interrupt);
    *IDT.lock() = Some(idt);
    // load it
    if let Some(ref i) = *IDT.lock() {
        i.load();
    }
}

pub fn remap_pic() {
    // Remap PIC to 0x20 and 0x28 (master 0x20-0x27, slave 0x28-0x2f)
    unsafe {
        let mut a1 = Port::<u8>::new(PIC1_DATA);
        let mut a2 = Port::<u8>::new(PIC2_DATA);
        // save masks
        let mask1 = a1.read();
        let mask2 = a2.read();

        let mut cmd1 = Port::<u8>::new(PIC1_COMMAND);
        let mut cmd2 = Port::<u8>::new(PIC2_COMMAND);

        // init command
        cmd1.write(0x11);
        cmd2.write(0x11);

        // set vector offsets
        a1.write(0x20); // master offset
        a2.write(0x28); // slave offset

        // tell master there is a slave at IRQ2 (0000 0100)
        a1.write(4);
        // tell slave its cascade identity (0000 0010)
        a2.write(2);

        // set environment info
        a1.write(0x01);
        a2.write(0x01);

        // restore masks
        a1.write(mask1);
        a2.write(mask2);
    }
}

/// Enable interrupts globally
pub fn enable_interrupts() {
    unsafe { interrupts::enable(); }
}

/// Send End Of Interrupt to PIC (for IRQs > 7 also send to slave)
fn send_eoi(irq: u8) {
    unsafe {
        let mut cmd = Port::<u8>::new(PIC1_COMMAND);
        if irq >= 8 {
            let mut s = Port::<u8>::new(PIC2_COMMAND);
            s.write(0x20);
        }
        cmd.write(0x20);
    }
}

/// Keyboard IRQ handler at vector 33 (0x21)
extern "x86-interrupt" fn keyboard_interrupt(_stack_frame: &mut InterruptStackFrame) {
    // read scancode from port 0x60
    unsafe {
        let mut port = Port::<u8>::new(0x60);
        let scancode: u8 = port.read();
        // push into kb queue (lock inside)
        Kb::push_scancode(scancode);
    }
    // send EOI
    send_eoi(1);
          }
