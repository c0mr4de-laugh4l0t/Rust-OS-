#[inline(always)]
pub fn prepare_stack(entry: extern "C" fn(), stack_base: usize, stack_size: usize) -> usize {
    // Start at top of stack, align to 16 bytes
    let mut sp = (stack_base + stack_size) & !0xF;

    unsafe {
        // Return address (what "ret" will jump to)
        sp -= core::mem::size_of::<usize>();
        (sp as *mut usize).write_volatile(entry as usize);

        // RBP (frame pointer)
        sp -= core::mem::size_of::<usize>();
        (sp as *mut usize).write_volatile(0);

        // Callee-saved registers (rbx, r12, r13, r14, r15)
        for _ in 0..5 {
            sp -= core::mem::size_of::<usize>();
            (sp as *mut usize).write_volatile(0);
        }
    }

    sp
}

