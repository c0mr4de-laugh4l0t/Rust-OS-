// syscall.rs
#![no_std]

use crate::process;
use crate::vga;
use core::str;

pub const SYS_EXIT: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_GETPID: u64 = 2;
pub const SYS_SPAWN: u64 = 3;
pub const SYS_YIELD: u64 = 4;
pub const SYS_SLEEP_MS: u64 = 5;

/// Simple kernel syscall dispatcher function.
/// NOTE: This is callable from kernel-mode tasks as a function.
/// Args are passed here as integers (caller will call syscall::call(...)).
pub fn dispatch(sysno: u64, arg0: usize, arg1: usize, arg2: usize) -> isize {
    match sysno {
        SYS_EXIT => {
            let pid = arg0 as u32;
            if process::exit_self(pid) {
                0
            } else {
                -1
            }
        }
        SYS_WRITE => {
            // fd = arg0, buf = arg1, len = arg2
            let fd = arg0 as u32;
            let buf_ptr = arg1 as *const u8;
            let len = arg2 as usize;
            if fd != 1 {
                return -1; // only stdout supported
            }
            if buf_ptr.is_null() || len == 0 {
                return 0;
            }
            // SAFETY: in Option A we assume processes are kernel-mode and use identity mapping.
            unsafe {
                let slice = core::slice::from_raw_parts(buf_ptr, len);
                // attempt to interpret as UTF-8; print directly as bytes otherwise
                match str::from_utf8(slice) {
                    Ok(s) => {
                        crate::vga::vprintln!("{}", s);
                    }
                    Err(_) => {
                        // print bytes hex as fallback
                        let mut out = [0u8; 256];
                        let take = core::cmp::min(len, out.len());
                        out[..take].copy_from_slice(&slice[..take]);
                        if let Ok(s) = str::from_utf8(&out[..take]) {
                            crate::vga::vprintln!("{}", s);
                        }
                    }
                }
            }
            len as isize
        }
        SYS_GETPID => {
            match process::current_pid() {
                Some(pid) => pid as isize,
                None => -1,
            }
        }
        SYS_SPAWN => {
            // arg0 = entry ptr (function pointer), arg1 = pages, arg2 ignored
            let entry_fn = arg0 as usize;
            let pages = arg1 as usize;
            if entry_fn == 0 {
                return -1;
            }
            // transmute usize->fn pointer
            let entry: extern "C" fn() = unsafe { core::mem::transmute(entry_fn) };
            if let Some(pid) = process::spawn(entry, pages, process::current_pid()) {
                pid as isize
            } else {
                -1
            }
        }
        SYS_YIELD => {
            crate::scheduler::task_yield();
            0
        }
        SYS_SLEEP_MS => {
            let pid = process::current_pid().unwrap_or(0);
            let ms = arg0 as u64;
            process::sleep_ms(pid, ms);
            // yield so we don't busy-loop
            crate::scheduler::task_yield();
            0
        }
        _ => -1,
    }
}

/// Small convenience wrapper for code inside kernel tasks to call a syscall.
/// Example: let r = syscall::call(syscall::SYS_GETPID, 0, 0, 0);
pub fn call(sysno: u64, a0: usize, a1: usize, a2: usize) -> isize {
    dispatch(sysno, a0, a1, a2)
}