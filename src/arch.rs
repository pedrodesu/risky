use alloc::boxed::Box;
use core::arch::asm;

use spin::{Mutex, Once};

use crate::task::Scheduler;

pub mod cause
{
    pub mod interrupts
    {
        pub const SUPERVISOR_SOFTWARE_INTERRUPT: usize = 1;
        pub const SUPERVISOR_TIMER_INTERRUPT: usize = 5;
    }

    pub mod exceptions
    {
        pub const INSTRUCTION_ACCESS_FAULT: usize = 1;
        pub const ILLEGAL_INSTRUCTION: usize = 2;
        pub const LOAD_ACCESS_FAULT: usize = 5;
        pub const STORE_ACCESS_FAULT: usize = 7;

        pub const USER_ECALL: usize = 8;
        pub const SUPERVISOR_ECALL: usize = 9;
        pub const MACHINE_ECALL: usize = 11;
    }
}

pub static CPU_VEC: Once<Box<[Cpu]>> = Once::new();

#[repr(C)]
pub struct Cpu
{
    pub physical_id: usize,
    pub logical_id: usize,
    pub scheduler: Mutex<Scheduler>,
    pub stack_top: usize,
    pub trap_stack_top: usize,
}

impl Cpu
{
    #[inline]
    pub fn set(&self)
    {
        let ptr = self as *const Cpu as usize;
        unsafe { asm!("mv tp, {0}", in(reg) ptr) }
    }

    #[inline]
    pub fn get() -> &'static Cpu
    {
        let ptr: usize;
        unsafe {
            asm!("mv {0}, tp", out(reg) ptr);
            &*(ptr as *const Cpu)
        }
    }
}
