pub fn prepare_stack(entry: extern "C" fn(), stack_base: usize, stack_size: usize) -> usize {
    let mut sp = stack_base + stack_size;
    sp &= !0xF;

    unsafe {
        // push entry return address
        sp -= core::mem::size_of::<usize>();
        (sp as *mut usize).write_volatile(entry as usize);

        // push rbp placeholder
        sp -= core::mem::size_of::<usize>();
        (sp as *mut usize).write_volatile(0usize);

        // push rbx, r12, r13, r14, r15 placeholders in reverse order so pop restores correctly
        sp -= core::mem::size_of::<usize>(); (sp as *mut usize).write_volatile(0usize); // rbx
        sp -= core::mem::size_of::<usize>(); (sp as *mut usize).write_volatile(0usize); // r12
        sp -= core::mem::size_of::<usize>(); (sp as *mut usize).write_volatile(0usize); // r13
        sp -= core::mem::size_of::<usize>(); (sp as *mut usize).write_volatile(0usize); // r14
        sp -= core::mem::size_of::<usize>(); (sp as *mut usize).write_volatile(0usize); // r15
    }
    sp
}
//switch so it is r15 down to r12
