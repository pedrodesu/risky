//! Defines `Context`, which stores the register state of a task.
//! This structure is carefully laid out to be compatible with the
//! `switch_context` assembly routine, enabling the saving and restoring of a
//! task's execution state.

use core::arch::naked_asm;

// We `derive(Default)` because we want to use a mock Context for the
// `idle_context`. It will then be populated with values via `ld` instructions
// on `switch_context`
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default)]
pub struct Context
{
    pub ra: usize, // Return address
    pub sp: usize, // Stack pointer
    // Frame pointers
    pub s0: usize,
    pub s1: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub pc: usize, // The hardware ret. Get out of the trap and into the task
}

#[unsafe(naked)]
pub unsafe extern "C" fn switch_context(old_ptr: *mut Context, new_ptr: *const Context)
{
    naked_asm!(
        // Save callee-saved registers of the old task
        "sd ra,   0*8(a0)",
        "sd sp,   1*8(a0)",
        "sd s0,   2*8(a0)",
        "sd s1,   3*8(a0)",
        "sd s2,   4*8(a0)",
        "sd s3,   5*8(a0)",
        "sd s4,   6*8(a0)",
        "sd s5,   7*8(a0)",
        "sd s6,   8*8(a0)",
        "sd s7,   9*8(a0)",
        "sd s8,  10*8(a0)",
        "sd s9,  11*8(a0)",
        "sd s10, 12*8(a0)",
        "sd s11, 13*8(a0)",
        // Restore callee-saved registers of the new task
        "ld ra,   0*8(a1)",
        "ld sp,   1*8(a1)",
        "ld s0,   2*8(a1)",
        "ld s1,   3*8(a1)",
        "ld s2,   4*8(a1)",
        "ld s3,   5*8(a1)",
        "ld s4,   6*8(a1)",
        "ld s5,   7*8(a1)",
        "ld s6,   8*8(a1)",
        "ld s7,   9*8(a1)",
        "ld s8,  10*8(a1)",
        "ld s9,  11*8(a1)",
        "ld s10, 12*8(a1)",
        "ld s11, 13*8(a1)",
        "ret"
    )
}
