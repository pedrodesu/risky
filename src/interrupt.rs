//! This module handles CPU interrupts and exceptions for the RISC-V machine
//! mode. It sets up the machine trap vector (`mtvec`) to point to an assembly
//! routine (`_trap`). This routine saves context, calls a high-level Rust
//! handler (`trap_handler`), and then restores context before returning.

use core::arch::{asm, global_asm};

use crate::{
    arch::cause::{self, exceptions, interrupts},
    plic, soc,
    task::Scheduler,
    timer, uart,
};

const MIE_FLAG: usize = 1 << 3; // Machine Interrupt Enable for `mstatus`

#[cfg(target_pointer_width = "64")]
global_asm!(include_str!("interrupt/rv64.S"));

#[cfg(target_pointer_width = "32")]
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
    mepc: usize,
    mcause: usize,
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

    let &mut TrapFrame { mcause, mepc, .. } = frame;

    let is_interrupt = mcause & CAUSE_INTERRUPT_FLAG != 0;
    // Mask out the interrupt bit to get the Exception Code
    let code = mcause & !CAUSE_INTERRUPT_FLAG;

    let new_epc = if is_interrupt
    {
        use interrupts::*;

        match code
        {
            MACHINE_TIMER_INTERRUPT => handle_timer_interrupt(mepc),
            MACHINE_EXTERNAL_INTERRUPT =>
            {
                handle_external_interrupt();
                mepc
            }
            _ => mepc,
        }
    }
    else
    {
        handle_exception(code, mepc)
    };

    frame.mepc = new_epc;
}

fn handle_timer_interrupt(epc: usize) -> usize
{
    timer::schedule_next();
    Scheduler::schedule(epc)
}

fn handle_external_interrupt()
{
    let irq = plic::claim();

    match irq
    {
        soc::uart::IRQ =>
        {
            if let Some(c) = uart::get_char()
            {
                // Echo back
                print!("{}", c as char);
            }
        }
        0 =>
        {}
        _ => panic!("Unhandled external IRQ: {}", irq),
    }

    if irq != 0
    {
        plic::complete(irq);
    }
}

fn handle_exception(code: usize, epc: usize) -> usize
{
    use exceptions::*;

    match code
    {
        USER_ECALL | SUPERVISOR_ECALL | MACHINE_ECALL =>
        {
            // Return next instruction address
            epc + 4
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

/// Consequently enable all the set interrupts with `init`
#[inline]
pub fn enable()
{
    // mstatus.MIE: Global interrupt enable for Machine Mode
    unsafe { csr_set_i!("mstatus", MIE_FLAG) }
}

/// Initialize Machine-Mode Interrupts
pub fn init(kernel_sp: usize)
{
    // mtvec setup: Direct mode
    // All traps will jump to the exact address of _trap
    unsafe {
        asm!(
            "la t0, {trap}",
            "csrw mtvec, t0",
            trap = sym _trap
        )
    }

    // Before enabling interrupts, mscratch MUST hold the kernel stack pointer
    unsafe { csr_write!("mscratch", kernel_sp) }

    // mie: Enable specific interrupt sources
    unsafe {
        csr_set!(
            "mie",
            1 << cause::interrupts::MACHINE_EXTERNAL_INTERRUPT
                | 1 << cause::interrupts::MACHINE_TIMER_INTERRUPT
                | 1 << cause::interrupts::MACHINE_SOFTWARE_INTERRUPT
        )
    }
}
