//! CPU-local architecture state and trap-cause constants.
//!
//! This module defines architecture-facing CPU metadata and constants used by
//! the rest of the kernel.

use alloc::{alloc::alloc, boxed::Box};
use core::{alloc::Layout, arch::asm};

use ::fdt::Fdt;
use spin::{Mutex, Once};

use crate::{
    STACK_SIZE, TRAP_STACK_SIZE, fdt,
    task::{Scheduler, Task},
};

macro_rules! define_page_config {
    ($size:literal) => {
        pub const PAGE_SIZE: usize = $size;

        #[repr(align($size))]
        pub struct PageAligned<const N: usize>(pub [u8; N]);
    };
}

define_page_config!(4096);

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
    /// Initialize per-hart metadata and stacks from the device tree.
    pub fn init_vec(dev_tree: &Fdt, boot_hart_id: usize)
    {
        let count = fdt::harts::parse_hart_count(dev_tree, boot_hart_id).unwrap();
        let cpus = (0..count)
            .map(|i| {
                let [stack_ptr, trap_stack_ptr] = [STACK_SIZE, TRAP_STACK_SIZE]
                    // Ensure page alignment
                    .map(|s| Layout::from_size_align(s, PAGE_SIZE).unwrap())
                    .map(|l| unsafe { alloc(l) as usize });

                Cpu {
                    physical_id: fdt::harts::to_physical(i),
                    logical_id: i,
                    scheduler: Mutex::new(Scheduler::with_task(Task::main())),
                    stack_top: stack_ptr + STACK_SIZE,
                    trap_stack_top: trap_stack_ptr + STACK_SIZE,
                }
            })
            .collect();

        CPU_VEC.call_once(|| cpus);
    }

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

    #[inline]
    pub fn nth(logical_id: usize) -> &'static Cpu
    {
        let cpus = CPU_VEC.wait();
        &cpus[logical_id]
    }
}
