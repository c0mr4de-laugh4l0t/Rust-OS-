#![no_std]

use core::str;
use core::sync::atomic::{AtomicIsize, Ordering};

pub const SYS_EXIT: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_GETPID: u64 = 2;
pub const SYS_SPAWN: u64 = 3;
pub const SYS_YIELD: u64 = 4;
pub const SYS_SLEEP_MS: u64 = 5;

pub fn dispatch(sysno: u64, a0: usize, a1: usize, a2: usize) -> isize {
    match sysno {
        SYS_EXIT => {
            let pid = a0 as u32;
            if crate::process::exit_self(pid) { 0 } else { -1 }
        }
        SYS_WRITE => {
            let fd = a0 as u32;
            let buf = a1 as *const u8;
            let len = a2 as usize;
            if fd != 1 { return -1; }
            if buf.is_null() || len == 0 { return 0; }
            unsafe {
                let slice = core::slice::from_raw_parts(buf, len);
                if let Ok(s) = str::from_utf8(slice) {
                    crate::vga::vprintln!("{}", s);
                } else {
                    let mut out = [0u8; 256];
                    let take = core::cmp::min(len, out.len());
                    out[..take].copy_from_slice(&slice[..take]);
                    if let Ok(s) = str::from_utf8(&out[..take]) {
                        crate::vga::vprintln!("{}", s);
                    }
                }
            }
            len as isize
        }
        SYS_GETPID => {
            match crate::process::current_pid() {
                Some(p) => p as isize,
                None => -1,
            }
        }
        SYS_SPAWN => {
            let entry = a0 as usize;
            let pages = a1 as usize;
            if entry == 0 { return -1; }
            let f: extern "C" fn() = unsafe { core::mem::transmute(entry) };
            if let Some(pid) = crate::process::spawn(f, pages, crate::process::current_pid()) {
                pid as isize
            } else { -1 }
        }
        SYS_YIELD => {
            crate::scheduler::task_yield();
            0
        }
        SYS_SLEEP_MS => {
            let ms = a0 as u64;
            let pid = crate::process::current_pid().unwrap_or(0);
            crate::process::sleep_ms(pid, ms);
            crate::scheduler::task_yield();
            0
        }
        _ => -1,
    }
}

pub fn call(sysno: u64, a0: usize, a1: usize, a2: usize) -> isize {
    dispatch(sysno, a0, a1, a2)
}