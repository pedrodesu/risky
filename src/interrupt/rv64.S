.section .text
.global _trap
.align 4
_trap:
    # sp = trap stack (the ptr you passed to interrupt::init)
    # sscratch = the stack we were using before the trap (kmain or task stack)
    csrrw sp, sscratch, sp

    # TrapFrame = TrapContext (30 words) + scause + padding = 32 words
    # 32 * 8 = 256 bytes
    addi sp, sp, -256

    # Save general-purpose registers into TrapContext
    sd ra,   0*8(sp)
    sd t0,   2*8(sp)
    sd t1,   3*8(sp)
    sd t2,   4*8(sp)
    sd t3,   5*8(sp)
    sd t4,   6*8(sp)
    sd t5,   7*8(sp)
    sd t6,   8*8(sp)
    sd a0,   9*8(sp)
    sd a1,  10*8(sp)
    sd a2,  11*8(sp)
    sd a3,  12*8(sp)
    sd a4,  13*8(sp)
    sd a5,  14*8(sp)
    sd a6,  15*8(sp)
    sd a7,  16*8(sp)
    sd s0,  17*8(sp)
    sd s1,  18*8(sp)
    sd s2,  19*8(sp)
    sd s3,  20*8(sp)
    sd s4,  21*8(sp)
    sd s5,  22*8(sp)
    sd s6,  23*8(sp)
    sd s7,  24*8(sp)
    sd s8,  25*8(sp)
    sd s9,  26*8(sp)
    sd s10, 27*8(sp)
    sd s11, 28*8(sp)

    # Save interrupted SP, EPC, and cause
    csrr t0, sscratch
    sd   t0,  1*8(sp)   # context.sp
    csrr t0, sepc
    sd   t0, 29*8(sp)   # context.pc
    csrr t0, scause
    sd   t0, 30*8(sp)   # frame.scause

    # Call Rust handler (pass sp as pointer to TrapFrame)
    mv   a0, sp
    call trap_handler

    # Restore CSRs from selected context
    ld   t0, 29*8(sp)
    csrw sepc, t0
    ld   t0,  1*8(sp)
    csrw sscratch, t0

    # Restore general-purpose registers from TrapContext
    ld ra,   0*8(sp)
    ld t0,   2*8(sp)
    ld t1,   3*8(sp)
    ld t2,   4*8(sp)
    ld t3,   5*8(sp)
    ld t4,   6*8(sp)
    ld t5,   7*8(sp)
    ld t6,   8*8(sp)
    ld a0,   9*8(sp)
    ld a1,  10*8(sp)
    ld a2,  11*8(sp)
    ld a3,  12*8(sp)
    ld a4,  13*8(sp)
    ld a5,  14*8(sp)
    ld a6,  15*8(sp)
    ld a7,  16*8(sp)
    ld s0,  17*8(sp)
    ld s1,  18*8(sp)
    ld s2,  19*8(sp)
    ld s3,  20*8(sp)
    ld s4,  21*8(sp)
    ld s5,  22*8(sp)
    ld s6,  23*8(sp)
    ld s7,  24*8(sp)
    ld s8,  25*8(sp)
    ld s9,  26*8(sp)
    ld s10, 27*8(sp)
    ld s11, 28*8(sp)

    # Clean up and swap back to interrupted stack
    addi sp, sp, 256
    csrrw sp, sscratch, sp
    sret
