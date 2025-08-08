.text
    .global context_switch
    .type context_switch, @function
// void context_switch(uintptr_t *old_rsp_ptr, uintptr_t new_rsp);
context_switch:
    // rdi = old_rsp_ptr, rsi = new_rsp

    // save callee-saved registers onto current stack in the order we will later restore
    pushq %rbp
    pushq %rbx
    pushq %r12
    pushq %r13
    pushq %r14
    pushq %r15

    // store current rsp into *old_rsp_ptr
    movq %rsp, (%rdi)

    // switch to new stack
    movq %rsi, %rsp

    // restore callee-saved regs from new stack in reverse order of pushes above
    popq %r15
    popq %r14
    popq %r13
    popq %r12
    popq %rbx
    popq %rbp

    // return: this will pop the return address we pushed (entry) and jump to it
    ret
