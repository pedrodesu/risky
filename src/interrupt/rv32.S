.section .text
.global _trap
.align 4
_trap:
    # sp = trap stack (the ptr you passed to interrupt::init)
    # sscratch = the stack we were using before the trap (kmain or task stack)
    csrrw sp, sscratch, sp

    # TrapFrame = TrapContext (30 words) + scause + padding = 32 words
    # 32 * 4 = 128 bytes
    addi sp, sp, -128

    # Save general-purpose registers into TrapContext
    sw ra,   0*4(sp)
    sw t0,   2*4(sp)
    sw t1,   3*4(sp)
    sw t2,   4*4(sp)
    sw t3,   5*4(sp)
    sw t4,   6*4(sp)
    sw t5,   7*4(sp)
    sw t6,   8*4(sp)
    sw a0,   9*4(sp)
    sw a1,  10*4(sp)
    sw a2,  11*4(sp)
    sw a3,  12*4(sp)
    sw a4,  13*4(sp)
    sw a5,  14*4(sp)
    sw a6,  15*4(sp)
    sw a7,  16*4(sp)
    sw s0,  17*4(sp)
    sw s1,  18*4(sp)
    sw s2,  19*4(sp)
    sw s3,  20*4(sp)
    sw s4,  21*4(sp)
    sw s5,  22*4(sp)
    sw s6,  23*4(sp)
    sw s7,  24*4(sp)
    sw s8,  25*4(sp)
    sw s9,  26*4(sp)
    sw s10, 27*4(sp)
    sw s11, 28*4(sp)

    # Save interrupted SP, EPC, and cause
    csrr t0, sscratch
    sw   t0,  1*4(sp)   # context.sp
    csrr t0, sepc
    sw   t0, 29*4(sp)   # context.pc
    csrr t0, scause
    sw   t0, 30*4(sp)   # frame.scause

    # Call Rust handler (pass sp as pointer to TrapFrame)
    mv   a0, sp
    call trap_handler

    # Restore CSRs from selected context
    lw   t0, 29*4(sp)
    csrw sepc, t0
    lw   t0,  1*4(sp)
    csrw sscratch, t0

    # Restore general-purpose registers from TrapContext
    lw ra,   0*4(sp)
    lw t0,   2*4(sp)
    lw t1,   3*4(sp)
    lw t2,   4*4(sp)
    lw t3,   5*4(sp)
    lw t4,   6*4(sp)
    lw t5,   7*4(sp)
    lw t6,   8*4(sp)
    lw a0,   9*4(sp)
    lw a1,  10*4(sp)
    lw a2,  11*4(sp)
    lw a3,  12*4(sp)
    lw a4,  13*4(sp)
    lw a5,  14*4(sp)
    lw a6,  15*4(sp)
    lw a7,  16*4(sp)
    lw s0,  17*4(sp)
    lw s1,  18*4(sp)
    lw s2,  19*4(sp)
    lw s3,  20*4(sp)
    lw s4,  21*4(sp)
    lw s5,  22*4(sp)
    lw s6,  23*4(sp)
    lw s7,  24*4(sp)
    lw s8,  25*4(sp)
    lw s9,  26*4(sp)
    lw s10, 27*4(sp)
    lw s11, 28*4(sp)

    # Clean up and swap back to interrupted stack
    addi sp, sp, 128
    csrrw sp, sscratch, sp
    sret
