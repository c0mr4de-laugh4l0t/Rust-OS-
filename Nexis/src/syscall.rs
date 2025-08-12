#![no_std]

pub fn call(num: u64, a1: usize, a2: usize, a3: usize) -> usize {
    match num {
        0 => sys_write(a1 as *const u8, a2),
        1 => sys_exit(a1 as i32),
        2 => crate::fs::fs_open(a1 as *const u8, a2),
        3 => crate::fs::fs_read(a1, a2 as *mut u8, a3),
        4 => crate::fs::fs_write(a1, a2 as *const u8, a3),
        5 => crate::fs::fs_close(a1),
        6 => crate::fs::fs_create(a1 as *const u8, a2, a3 as *const u8, 0),
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