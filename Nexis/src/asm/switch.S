/* switch.S - GAS syntax (AT&T) */

    .section .text
    .global context_switch
    .type context_switch, @function
context_switch:
    /* args: rdi = pointer to old_rsp (usize*) ; rsi = new_rsp (usize) */
    /* store current rsp into *rdi, then load new rsp and ret */
    pushq %rbp
    movq %rsp, %rbp

    /* save current rsp into memory pointed by rdi */
    movq %rsi, %rax   /* temp: put new_rsp into rax for later */
    movq %rsp, (%rdi) /* *old_rsp_ptr = rsp */

    /* set new stack */
    movq %rax, %rsp

    popq %rbp
    ret

    .global enter_user
    .type enter_user, @function
/* enter_user(rip, rsp) */
enter_user:
    /* rdi = rip, rsi = rsp */
    /* This will prepare an iretq frame for switching to ring 3.
       Make sure GDT selectors for user code/data are 0x1B and 0x23.
    */
    cli
    swapgs               /* if using %gs for CPU-local data; depends on setup */

    /* push user SS, RSP, RFLAGS, CS, RIP (in that order) */
    movq %rsi, %rax      /* rax = user_rsp */
    pushq $0x23          /* user SS selector */
    pushq %rax           /* user RSP */
    pushfq
    pushq $0x1B          /* user CS selector */
    pushq %rdi           /* user RIP */
    iretq
    /* never returns */
