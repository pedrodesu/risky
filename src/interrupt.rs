//! Trap/interrupt initialization and dispatch for supervisor mode.
//!
//! This module owns trap setup and high-level interrupt/exception handling.

use core::arch::{asm, global_asm};

use crate::{
    arch::{
        Cpu,
        cause::{self, exceptions, interrupts},
    },
    platform::timer,
    task::TrapContext,
};

pub const SIE_FLAG: usize = 1 << 1; // Supervisor Interrupt Enable for `sstatus`

pub struct LocalIrqGuard
{
    was_enabled: bool,
}

impl Drop for LocalIrqGuard
{
    fn drop(&mut self)
    {
        if self.was_enabled
        {
            enable();
        }
    }
}

#[cfg(target_arch = "riscv64")]
global_asm!(include_str!("interrupt/rv64.S"));

#[cfg(target_arch = "riscv32")]
global_asm!(include_str!("interrupt/rv32.S"));

/// Trap frame layout shared with `interrupt/rv*.S`.
#[repr(C)]
struct TrapFrame
{
    context: TrapContext,
    scause: usize,
    _reserved: usize,
}

// Low-level trap entry point referenced by `stvec`.
unsafe extern "C" {
    fn _trap();
}

/// The high-level trap dispatcher
#[unsafe(no_mangle)]
extern "C" fn trap_handler(frame: &mut TrapFrame)
{
    // Check top bit. If it's 1, we have an interrupt. Otherwise, it's an exception.
    const CAUSE_INTERRUPT_FLAG: usize = 1 << (core::mem::size_of::<usize>() * 8 - 1);

    let scause = frame.scause;

    let is_interrupt = scause & CAUSE_INTERRUPT_FLAG != 0;
    // Mask out the interrupt bit to get the Exception Code
    let code = scause & !CAUSE_INTERRUPT_FLAG;

    if is_interrupt
    {
        use interrupts::*;

        match code
        {
            SUPERVISOR_SOFTWARE_INTERRUPT => handle_software_interrupt(frame),
            SUPERVISOR_TIMER_INTERRUPT => handle_timer_interrupt(frame),
            _ =>
            {}
        }
    }
    else
    {
        handle_exception(code, frame)
    }
}

fn handle_software_interrupt(frame: &mut TrapFrame)
{
    timer::ipi::clear();

    let mut scheduler = Cpu::get().scheduler.lock();
    scheduler.schedule(&mut frame.context)
}

fn handle_timer_interrupt(frame: &mut TrapFrame)
{
    timer::schedule_next();

    let mut scheduler = Cpu::get().scheduler.lock();
    scheduler.schedule(&mut frame.context)
}

fn handle_exception(code: usize, frame: &mut TrapFrame)
{
    use exceptions::*;

    match code
    {
        USER_ECALL | SUPERVISOR_ECALL | MACHINE_ECALL =>
        {
            let mut scheduler = Cpu::get().scheduler.lock();

            // We move the EPC forward by 4 so that IF this task is ever
            // rescheduled (not applicable for Dead tasks, but vital for Syscalls),
            // it resumes AFTER the ecall instruction.
            frame.context.pc += 4;

            scheduler.schedule(&mut frame.context)
        }
        INSTRUCTION_ACCESS_FAULT => panic!(
            "Instruction Access Fault at {:#x}! (Likely task returned or bad RA)",
            frame.context.pc
        ),
        ILLEGAL_INSTRUCTION => panic!("Illegal Instruction at {:#x}!", frame.context.pc),
        LOAD_ACCESS_FAULT => panic!("Load Access Fault at {:#x}!", frame.context.pc),
        STORE_ACCESS_FAULT => panic!("Store Access Fault at {:#x}!", frame.context.pc),
        _ => panic!(
            "Unhandled exception: code {}, epc {:#x}",
            code, frame.context.pc
        ),
    }
}

/// Enable local supervisor interrupts on the current hart.
#[inline]
pub fn enable()
{
    unsafe { csr_set_i!("sstatus", SIE_FLAG) }
}

/// Returns whether local supervisor interrupts are currently enabled.
#[inline]
pub fn is_enabled() -> bool
{
    unsafe { csr_read!("sstatus") & SIE_FLAG != 0 }
}

/// Disable local supervisor interrupts on the current hart.
#[inline]
pub fn disable()
{
    unsafe { csr_clear_i!("sstatus", SIE_FLAG) }
}

/// Disable local interrupts and restore the previous interrupt state on drop.
#[inline]
pub fn disable_guard() -> LocalIrqGuard
{
    let was_enabled = is_enabled();
    disable();
    LocalIrqGuard { was_enabled }
}

/// Run `f` with local interrupts disabled, then restore the previous state.
#[inline]
pub fn with_disabled<T>(f: impl FnOnce() -> T) -> T
{
    let _guard = disable_guard();
    f()
}

/// Configure trap vector and enabled interrupt sources for this hart.
pub fn init(trap_stack_ptr: usize)
{
    // stvec setup: direct mode.
    unsafe {
        asm!(
            "la t0, {trap}",
            "csrw stvec, t0",
            trap = sym _trap
        )
    }

    // `sscratch` holds the trap stack pointer for the assembly prologue.
    unsafe { csr_write!("sscratch", trap_stack_ptr) }

    // Enable supervisor timer and software interrupts.
    unsafe {
        csr_set!(
            "sie",
            1 << cause::interrupts::SUPERVISOR_TIMER_INTERRUPT
                | 1 << cause::interrupts::SUPERVISOR_SOFTWARE_INTERRUPT
        )
    }
}
