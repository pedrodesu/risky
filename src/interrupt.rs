//! This module handles CPU interrupts and exceptions for the RISC-V machine
//! mode. It sets up the machine trap vector (`mtvec`) to point to an assembly
//! routine (`_trap`). This routine saves context, calls a high-level Rust
//! handler (`trap_handler`), and then restores context before returning.

use core::arch::{asm, global_asm};

use crate::{
    arch::{
        Cpu,
        cause::{self, exceptions, interrupts},
    },
    timer,
};

pub const SIE_FLAG: usize = 1 << 1; // Supervisor Interrupt Enable for `sstatus`

#[cfg(target_arch = "riscv64")]
global_asm!(include_str!("interrupt/rv64.S"));

#[cfg(target_arch = "riscv32")]
global_asm!(include_str!("interrupt/rv32.S"));

/// Defines the layout of the registers saved on the stack during a trap.
/// This structure is accessed from both Rust and assembly.
#[repr(C)]
struct TrapFrame
{
    // General-purpose registers
    ra: usize,
    t0: usize,
    t1: usize,
    t2: usize,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,
    t3: usize,
    t4: usize,
    t5: usize,
    t6: usize,
    // CSRs
    sepc: usize,
    scause: usize,
    sscratch: usize,
}

// Low-level trap entry point.
// Referenced by `mtvec` (Machine Trap-Vector Base-Address Register).
unsafe extern "C" {
    fn _trap();
}

/// The high-level trap dispatcher
#[unsafe(no_mangle)]
extern "C" fn trap_handler(frame: &mut TrapFrame)
{
    // Check top bit. If it's 1, we have an interrupt. Otherwise, it's an exception.
    const CAUSE_INTERRUPT_FLAG: usize = 1 << (core::mem::size_of::<usize>() * 8 - 1);

    let &mut TrapFrame { scause, sepc, .. } = frame;

    let is_interrupt = scause & CAUSE_INTERRUPT_FLAG != 0;
    // Mask out the interrupt bit to get the Exception Code
    let code = scause & !CAUSE_INTERRUPT_FLAG;

    let new_epc = if is_interrupt
    {
        use interrupts::*;

        match code
        {
            SUPERVISOR_SOFTWARE_INTERRUPT => handle_software_interrupt(sepc),
            SUPERVISOR_TIMER_INTERRUPT => handle_timer_interrupt(sepc),
            _ => sepc,
        }
    }
    else
    {
        handle_exception(code, sepc)
    };

    frame.sepc = new_epc;
}

fn handle_software_interrupt(epc: usize) -> usize
{
    timer::ipi::clear();

    let mut scheduler = Cpu::get().scheduler.lock();
    scheduler.schedule(epc)
}

fn handle_timer_interrupt(epc: usize) -> usize
{
    timer::schedule_next();

    let mut scheduler = Cpu::get().scheduler.lock();
    scheduler.schedule(epc)
}

fn handle_exception(code: usize, epc: usize) -> usize
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
            let next_pc = epc + 4;

            // If the task is Dead, schedule() returns the PC of a NEW task.
            // If the task is alive (e.g. a yield), it returns next_pc.
            scheduler.schedule(next_pc)
        }
        INSTRUCTION_ACCESS_FAULT => panic!(
            "Instruction Access Fault at {:#x}! (Likely task returned or bad RA)",
            epc
        ),
        ILLEGAL_INSTRUCTION => panic!("Illegal Instruction at {:#x}!", epc),
        LOAD_ACCESS_FAULT => panic!("Load Access Fault at {:#x}!", epc),
        STORE_ACCESS_FAULT => panic!("Store Access Fault at {:#x}!", epc),
        _ => panic!("Unhandled exception: code {}, epc {:#x}", code, epc),
    }
}

/// Enable all the set interrupts with `init`
#[inline]
pub fn enable()
{
    // mstatus.MIE: Global interrupt enable for Machine Mode
    unsafe { csr_set_i!("sstatus", SIE_FLAG) }
}

/// Disable all interrupts
#[inline]
pub fn disable()
{
    // mstatus.MIE: Global interrupt enable for Machine Mode
    unsafe { csr_clear_i!("sstatus", SIE_FLAG) }
}

/// Initialize Machine-Mode Interrupts
pub fn init(trap_stack_ptr: usize)
{
    // stvec setup: Direct mode
    // All traps will jump to the exact address of _trap
    unsafe {
        asm!(
            "la t0, {trap}",
            "csrw stvec, t0",
            trap = sym _trap
        )
    }

    // Before enabling interrupts, sscratch must hold the kernel stack pointer
    unsafe { csr_write!("sscratch", trap_stack_ptr) }

    // sie: Enable specific interrupt sources
    unsafe {
        csr_set!(
            "sie",
            1 << cause::interrupts::SUPERVISOR_TIMER_INTERRUPT
                | 1 << cause::interrupts::SUPERVISOR_SOFTWARE_INTERRUPT
        )
    }
}
