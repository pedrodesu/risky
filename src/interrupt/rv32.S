.section .text
.global _trap
.align 4
_trap:
    # sp = trap stack (the ptr you passed to interrupt::init)
    # sscratch = the stack we were using before the trap (kmain or task stack)
    csrrw sp, sscratch, sp

    # 19 registers * 4 bytes = 76 bytes
    # 80 bytes is the minimum for 16-byte alignment
    addi sp, sp, -80

    # Save General Purpose Registers
    sw ra,  0*4(sp)
    sw t0,  1*4(sp)
    sw t1,  2*4(sp)
    sw t2,  3*4(sp)
    sw a0,  4*4(sp)
    sw a1,  5*4(sp)
    sw a2,  6*4(sp)
    sw a3,  7*4(sp)
    sw a4,  8*4(sp)
    sw a5,  9*4(sp)
    sw a6, 10*4(sp)
    sw a7, 11*4(sp)
    sw t3, 12*4(sp)
    sw t4, 13*4(sp)
    sw t5, 14*4(sp)
    sw t6, 15*4(sp)

    # Save CSRs
    csrr t0, sepc
    sw   t0, 16*4(sp)
    csrr t0, scause
    sw   t0, 17*4(sp)
    csrr t0, sscratch
    sw   t0, 18*4(sp)

    # Call Rust handler (pass sp as pointer to TrapFrame)
    mv   a0, sp
    call trap_handler

    # Restore General Purpose Registers
    lw ra,  0*4(sp)
    lw t0,  1*4(sp)
    lw t1,  2*4(sp)
    lw t2,  3*4(sp)
    lw a0,  4*4(sp)
    lw a1,  5*4(sp)
    lw a2,  6*4(sp)
    lw a3,  7*4(sp)
    lw a4,  8*4(sp)
    lw a5,  9*4(sp)
    lw a6, 10*4(sp)
    lw a7, 11*4(sp)
    lw t3, 12*4(sp)
    lw t4, 13*4(sp)
    lw t5, 14*4(sp)
    lw t6, 15*4(sp)

    # Restore CSRs (mcause isn't usually restored as it's read-only/status)
    lw   t0, 16*4(sp)
    csrw sepc, t0
    lw   t0, 18*4(sp)
    csrw sscratch, t0

    # Clean up and swap back
    addi sp, sp, 80
    csrrw sp, sscratch, sp  # sp is back to original, sscratch is back to safe stack
    sret
