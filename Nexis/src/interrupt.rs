// Nexis/src/interrupts.rs
#![no_std]

use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use x86_64::instructions::interrupts;
use x86_64::instructions::port::Port;

use crate::kb::Kb;

pub const PIC1_COMMAND: u16 = 0x20;
pub const PIC1_DATA: u16 = 0x21;
pub const PIC2_COMMAND: u16 = 0xA0;
pub const PIC2_DATA: u16 = 0xA1;

lazy_static! {
    static ref IDT: Mutex<Option<InterruptDescriptorTable>> = Mutex::new(None);
}

pub fn init_idt() {
    let mut idt = InterruptDescriptorTable::new();
    idt[33].set_handler_fn(keyboard_interrupt); // keyboard
    idt[0x80].set_handler_fn(syscall_interrupt); // syscalls
    *IDT.lock() = Some(idt);
    if let Some(ref i) = *IDT.lock() {
        i.load();
    }
}

pub fn remap_pic() {
    unsafe {
        let mut a1 = Port::<u8>::new(PIC1_DATA);
        let mut a2 = Port::<u8>::new(PIC2_DATA);
        let mask1 = a1.read();
        let mask2 = a2.read();

        let mut cmd1 = Port::<u8>::new(PIC1_COMMAND);
        let mut cmd2 = Port::<u8>::new(PIC2_COMMAND);

        cmd1.write(0x11);
        cmd2.write(0x11);

        a1.write(0x20);
        a2.write(0x28);

        a1.write(4);
        a2.write(2);

        a1.write(0x01);
        a2.write(0x01);

        a1.write(mask1);
        a2.write(mask2);
    }
}

pub fn enable_interrupts() {
    unsafe { interrupts::enable(); }
}

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

extern "x86-interrupt" fn keyboard_interrupt(_stack_frame: &mut InterruptStackFrame) {
    unsafe {
        let mut port = Port::<u8>::new(0x60);
        let scancode: u8 = port.read();
        Kb::push_scancode(scancode);
    }
    send_eoi(1);
}

extern "x86-interrupt" fn syscall_interrupt(_stack_frame: &mut InterruptStackFrame) {
    use core::arch::asm;

    let num: usize;
    let a1: usize;
    let a2: usize;
    let a3: usize;

    unsafe {
        asm!("mov {}, rax", out(reg) num);
        asm!("mov {}, rdi", out(reg) a1);
        asm!("mov {}, rsi", out(reg) a2);
        asm!("mov {}, rdx", out(reg) a3);
    }

    let ret = crate::syscall::syscall_handler(num, a1, a2, a3);

    unsafe {
        asm!("mov rax, {}", in(reg) ret);
    }
}