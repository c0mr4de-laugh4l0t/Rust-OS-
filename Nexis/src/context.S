/* context.S - context switch for x86_64 System V ABI
   void context_switch(uintptr_t *old_rsp_ptr, uintptr_t new_rsp);
   Save callee-saved registers into old stack frame and store updated rsp into *old_rsp_ptr.
   Load new_rsp into rsp and restore callee-saved registers then return (ret).
*/

    .text
    .global context_switch
    .type context_switch, @function
context_switch:
    // rdi = old_rsp_ptr, rsi = new_rsp
    // Save callee-saved regs onto current stack
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

    // restore callee-saved regs from new stack (they were pushed there when preparing)
    popq %r15
    popq %r14
    popq %r13
    popq %r12
    popq %rbx
    popq %rbp

    // ret will jump to the address on top of new stack (prepare_stack wrote entry as return addr)
    ret
