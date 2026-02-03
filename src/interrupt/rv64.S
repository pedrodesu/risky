.section .text
.global _trap
.align 4
_trap:
    # sp = trap stack (the ptr you passed to interrupt::init)
    # mscratch = the stack we were using before the trap (kmain or task stack)
    csrrw sp, mscratch, sp

    # 19 registers * 8 bytes = 152 bytes
    # 160 bytes is the minimum for 16-byte alignment
    addi sp, sp, -160

    # Save General Purpose Registers
    sd ra,  0*8(sp)
    sd t0,  1*8(sp)
    sd t1,  2*8(sp)
    sd t2,  3*8(sp)
    sd a0,  4*8(sp)
    sd a1,  5*8(sp)
    sd a2,  6*8(sp)
    sd a3,  7*8(sp)
    sd a4,  8*8(sp)
    sd a5,  9*8(sp)
    sd a6, 10*8(sp)
    sd a7, 11*8(sp)
    sd t3, 12*8(sp)
    sd t4, 13*8(sp)
    sd t5, 14*8(sp)
    sd t6, 15*8(sp)

    # Save CSRs
    csrr t0, mepc
    sd   t0, 16*8(sp)
    csrr t0, mcause
    sd   t0, 17*8(sp)
    csrr t0, mscratch
    sd   t0, 18*8(sp)

    # Call Rust handler (pass sp as pointer to TrapFrame)
    mv   a0, sp
    call trap_handler

    # Restore General Purpose Registers
    ld ra,  0*8(sp)
    ld t0,  1*8(sp)
    ld t1,  2*8(sp)
    ld t2,  3*8(sp)
    ld a0,  4*8(sp)
    ld a1,  5*8(sp)
    ld a2,  6*8(sp)
    ld a3,  7*8(sp)
    ld a4,  8*8(sp)
    ld a5,  9*8(sp)
    ld a6, 10*8(sp)
    ld a7, 11*8(sp)
    ld t3, 12*8(sp)
    ld t4, 13*8(sp)
    ld t5, 14*8(sp)
    ld t6, 15*8(sp)

    # Restore CSRs (mcause isn't usually restored as it's read-only/status)
    ld   t0, 16*8(sp)
    csrw mepc, t0
    ld   t0, 18*8(sp)
    csrw mscratch, t0

    # Clean up and swap back
    addi sp, sp, 160
    csrrw sp, mscratch, sp  # sp is back to original, mscratch is back to safe stack
    mret
