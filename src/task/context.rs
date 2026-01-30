use core::arch::global_asm;

// We `derive(Default)` because we want to use a mock Context for the
// `idle_context`. It will then be populated with values via `ld` instructions
// on `switch_context`
// This struct and the fields order are tightly coupled with the .S assembly!
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default)]
pub struct Context
{
    // Return address
    pub ra: usize,
    // Stack pointer - This is why we align(16) the struct. RISC-V requires it.
    pub sp: usize,
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
    // The hardware ret. Get out of the trap and into the task
    pub pc: usize,
}

#[cfg(target_pointer_width = "64")]
global_asm!(include_str!("context/rv64.S"));

#[cfg(target_pointer_width = "32")]
global_asm!(include_str!("context/rv32.S"));

unsafe extern "C" {
    pub fn switch_context(old_ptr: *mut Context, new_ptr: *const Context);
}
