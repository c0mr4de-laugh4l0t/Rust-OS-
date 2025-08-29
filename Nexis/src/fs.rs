// Nexis/src/fs.rs
#![no_std]

use core::ptr;

const DEMO_FILES: [&str; 2] = ["readme.txt", "hello.txt"];
const DEMO_CONTENTS: [&str; 2] = [
    "This is a demo file system.\n",
    "Hello from Nexis FS layer!\n",
];

pub fn fs_init() {}

pub fn list_files_syscall(out_buf: *mut u8, out_buf_len: usize) -> usize {
    if out_buf.is_null() || out_buf_len == 0 {
        return 0;
    }
    let mut written = 0;
    for name in DEMO_FILES {
        let line = format!("{}\n", name);
        let bytes = line.as_bytes();
        for &b in bytes {
            if written >= out_buf_len {
                return written;
            }
            unsafe { ptr::write(out_buf.add(written), b); }
            written += 1;
        }
    }
    written
}

pub fn read_file_syscall(filename_ptr: *const u8, filename_len: usize, out_buf: *mut u8) -> usize {
    if filename_ptr.is_null() || out_buf.is_null() {
        return 0;
    }
    let name = unsafe {
        let slice = core::slice::from_raw_parts(filename_ptr, filename_len);
        core::str::from_utf8(slice).unwrap_or("")
    };
    for (i, fname) in DEMO_FILES.iter().enumerate() {
        if *fname == name {
            let data = DEMO_CONTENTS[i].as_bytes();
            for (j, &b) in data.iter().enumerate() {
                unsafe { ptr::write(out_buf.add(j), b); }
            }
            return data.len();
        }
    }
    0
}