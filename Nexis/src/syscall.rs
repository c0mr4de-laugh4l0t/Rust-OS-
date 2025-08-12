#![no_std]

use core::arch::asm;

pub type SyscallFn = fn(arg1: usize, arg2: usize, arg3: usize) -> usize;

static mut SYSCALL_TABLE: [Option<SyscallFn>; 64] = [None; 64];

pub fn register_syscall(num: usize, func: SyscallFn) {
    unsafe { SYSCALL_TABLE[num] = Some(func); }
}

#[no_mangle]
pub extern "C" fn syscall_handler(num: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
    unsafe {
        if let Some(f) = SYSCALL_TABLE.get(num).and_then(|f| *f) {
            f(arg1, arg2, arg3)
        } else {
            usize::MAX
        }
    }
}

pub unsafe fn do_syscall(num: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
    let ret: usize;
    asm!(
        "mov rax, {0}",
        "mov rdi, {1}",
        "mov rsi, {2}",
        "mov rdx, {3}",
        "int 0x80",
        in(reg) num, in(reg) arg1, in(reg) arg2, in(reg) arg3,
        lateout("rax") ret
    );
    ret
}