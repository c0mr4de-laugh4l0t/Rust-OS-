pub fn call(num: u64, a1: usize, a2: usize, a3: usize) -> usize {
    match num {
        0 => sys_write(a1 as *const u8, a2),
        1 => sys_exit(a1 as i32),
        _ => usize::MAX,
    }
}

fn sys_write(ptr: *const u8, len: usize) -> usize {
    unsafe {
        let slice = core::slice::from_raw_parts(ptr, len);
        if let Ok(s) = core::str::from_utf8(slice) {
            crate::vga::vprint!("{}", s);
            len
        } else {
            0
        }
    }
}

fn sys_exit(_code: i32) -> usize {
    loop {
        core::hint::spin_loop();
    }
}