pub fn write(s: &str) -> usize {
    unsafe {
        let ptr = s.as_ptr();
        let len = s.len();
        let ret: usize;
        asm!(
            "mov rax, 0",
            "mov rdi, {0}",
            "mov rsi, {1}",
            "mov rdx, 0",
            "int 0x80",
            in(reg) ptr,
            in(reg) len,
            out("rax") ret
        );
        ret
    }
}

pub fn exit(code: i32) -> ! {
    unsafe {
        asm!(
            "mov rax, 1",
            "mov rdi, {0}",
            "mov rsi, 0",
            "mov rdx, 0",
            "int 0x80",
            in(reg) code
        );
        core::hint::unreachable_unchecked();
    }
}