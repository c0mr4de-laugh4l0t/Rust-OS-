#![feature(asm_const)]  // if needed on nightly

/// Switch from the current task to another.
/// 
/// # Safety
/// - `old_rsp_ptr` must be a valid pointer to store the old rsp.
/// - `new_rsp` must be a valid stack pointer created by `prepare_stack`.
#[naked]
pub unsafe extern "C" fn context_switch(old_rsp_ptr: *mut usize, new_rsp: usize) {
    core::arch::asm!(
        // save callee-saved registers
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // store current rsp into *old_rsp_ptr
        "mov [rdi], rsp",

        // switch to new stack
        "mov rsp, rsi",

        // restore callee-saved registers
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",

        "ret",
        options(noreturn)
    )
}
