// Nexis/src/syscall.rs
#![no_std]

pub const SYS_WRITE: usize = 0;
pub const SYS_EXIT: usize = 1;
pub const SYS_LIST_FILES: usize = 2;
pub const SYS_READ_FILE: usize = 3;

pub fn syscall_handler(num: usize, a1: usize, a2: usize, a3: usize) -> usize {
    match num {
        SYS_WRITE => sys_write(a1 as *const u8, a2),
        SYS_EXIT => sys_exit(a1 as i32),
        SYS_LIST_FILES => crate::fs::list_files_syscall(a1 as *mut u8, a2),
        SYS_READ_FILE => crate::fs::read_file_syscall(a1 as *const u8, a2 as usize, a3 as *mut u8),
        _ => usize::MAX,
    }
}

fn sys_write(ptr: *const u8, len: usize) -> usize {
    if ptr.is_null() || len == 0 {
        return 0;
    }
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