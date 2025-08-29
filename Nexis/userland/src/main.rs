#![no_std]
#![no_main]

use core::panic::PanicInfo;

/// Syscall numbers must match kernel
const SYS_WRITE: usize = 0;
const SYS_EXIT: usize = 1;
const SYS_LIST_FILES: usize = 2;
const SYS_READ_FILE: usize = 3;

#[inline(always)]
fn sys_write(ptr: *const u8, len: usize) -> usize {
    let ret: usize;
    unsafe {
        core::arch::asm!(
            "mov rax, {num}",
            "mov rdi, {ptr}",
            "mov rsi, {len}",
            "int 0x80",
            num = const SYS_WRITE,
            ptr = in(reg) ptr,
            len = in(reg) len,
            out("rax") ret,
            options(nostack, preserves_flags),
        );
    }
    ret
}

#[inline(always)]
fn sys_exit(code: i32) -> ! {
    unsafe {
        core::arch::asm!(
            "mov rax, {num}",
            "mov rdi, {code}",
            "int 0x80",
            num = const SYS_EXIT,
            code = in(reg) code,
            options(noreturn),
        );
    }
}

/// Fill `out_buf` with newline-separated filenames; returns bytes_written
#[inline(always)]
fn sys_list_files(out_buf: *mut u8, out_buf_len: usize) -> usize {
    let ret: usize;
    unsafe {
        core::arch::asm!(
            "mov rax, {num}",
            "mov rdi, {ptr}",
            "mov rsi, {len}",
            "int 0x80",
            num = const SYS_LIST_FILES,
            ptr = in(reg) out_buf,
            len = in(reg) out_buf_len,
            out("rax") ret,
            options(nostack, preserves_flags),
        );
    }
    ret
}

/// Read filename (ptr/len) and copy file contents to out_buf ; returns bytes_read
#[inline(always)]
fn sys_read_file(fname_ptr: *const u8, fname_len: usize, out_buf: *mut u8) -> usize {
    let ret: usize;
    unsafe {
        core::arch::asm!(
            "mov rax, {num}",
            "mov rdi, {p1}", // filename ptr
            "mov rsi, {p2}", // filename len
            "mov rdx, {p3}", // out buf
            "int 0x80",
            num = const SYS_READ_FILE,
            p1 = in(reg) fname_ptr,
            p2 = in(reg) fname_len,
            p3 = in(reg) out_buf,
            out("rax") ret,
            options(nostack, preserves_flags),
        );
    }
    ret
}

/// Helper to call sys_write with a Rust string literal
fn write_str(s: &str) {
    let _ = sys_write(s.as_ptr(), s.len());
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // banner
    write_str("=== IronVeil (userland) — powered by Nexis ===\n\n");

    // list files into buffer
    let mut buf = [0u8; 512];
    let n = sys_list_files(buf.as_mut_ptr(), buf.len());
    if n > 0 && n <= buf.len() {
        // write the listing
        write_str("Files:\n");
        // write buffer (assume valid UTF-8 demo data)
        let s = unsafe { core::str::from_utf8_unchecked(&buf[..n]) };
        write_str(s);
        write_str("\n");
    } else {
        write_str("No files or listing failed.\n\n");
    }

    // try reading demo file "hello.txt"
    let fname = b"hello.txt";
    let mut out = [0u8; 512];
    let rn = sys_read_file(fname.as_ptr(), fname.len(), out.as_mut_ptr());
    if rn > 0 && rn <= out.len() {
        write_str("=== hello.txt ===\n");
        let s = unsafe { core::str::from_utf8_unchecked(&out[..rn]) };
        write_str(s);
        write_str("\n");
    } else {
        write_str("Could not read hello.txt\n");
    }

    write_str("\nUserland exiting.\n");
    sys_exit(0)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // best-effort print panic message
    write_str("userland panic\n");
    sys_exit(1)
          }
