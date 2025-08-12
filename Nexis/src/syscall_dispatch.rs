#![no_std]

pub fn sys_write(arg1: usize, _arg2: usize, _arg3: usize) -> usize {
    use crate::vga::VGA_WRITER;
    let s = unsafe { core::str::from_utf8_unchecked(core::slice::from_raw_parts(arg1 as *const u8, _arg2)) };
    VGA_WRITER.lock().write_str(s);
    0
}

pub fn sys_getpid(_arg1: usize, _arg2: usize, _arg3: usize) -> usize {
    42
}