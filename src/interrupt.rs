//! This module handles CPU interrupts and exceptions for the RISC-V machine
//! mode. It sets up the machine trap vector (`mtvec`) to point to an assembly
//! routine (`_trap`). This routine saves context, calls a high-level Rust
//! handler (`trap_handler`), and then restores context before returning.

use core::arch::{asm, naked_asm};

use crate::{plic, soc, task::Scheduler, timer, uart};

const CAUSE_INTERRUPT_FLAG: usize = 1 << 63;

const MIE_FLAG: usize = 1 << 3; // Machine Interrupt Enable for `mstatus`

const MEIE_FLAG: usize = 1 << 11; // External communication (UART for now)
const MTIE_FLAG: usize = 1 << 7; // Timer (CLINT in our case)
const MSIE_FLAG: usize = 1 << 3; // Software interrupt (Multicore communication (Scheduler for now))

/// Low-level trap entry point.
/// Referenced by `mtvec` (Machine Trap-Vector Base-Address Register).
#[unsafe(naked)]
extern "C" fn _trap()
{
    naked_asm!(
        "addi sp, sp, -128 # Create stack frame for trap context",
        "sd ra, 0*8(sp) # Store registers on the stack",
        "sd t0, 1*8(sp)",
        "sd t1, 2*8(sp)",
        "sd t2, 3*8(sp)",
        "sd a0, 4*8(sp)",
        "sd a1, 5*8(sp)",
        "sd a2, 6*8(sp)",
        "sd a3, 7*8(sp)",
        "sd a4, 8*8(sp)",
        "sd a5, 9*8(sp)",
        "sd a6, 10*8(sp)",
        "sd a7, 11*8(sp)",
        "sd t3, 12*8(sp)",
        "sd t4, 13*8(sp)",
        "sd t5, 14*8(sp)",
        "sd t6, 15*8(sp)",

        "csrr a0, mcause # Call trap handler",
        "csrr a1, mepc",
        "call {handler}",
        "csrw mepc, a0 # Set the return value of `trap_handler` as the new `epc`",

        "ld ra, 0*8(sp) # Restore registers back from the stack",
        "ld t0, 1*8(sp)",
        "ld t1, 2*8(sp)",
        "ld t2, 3*8(sp)",
        "ld a0, 4*8(sp)",
        "ld a1, 5*8(sp)",
        "ld a2, 6*8(sp)",
        "ld a3, 7*8(sp)",
        "ld a4, 8*8(sp)",
        "ld a5, 9*8(sp)",
        "ld a6, 10*8(sp)",
        "ld a7, 11*8(sp)",
        "ld t3, 12*8(sp)",
        "ld t4, 13*8(sp)",
        "ld t5, 14*8(sp)",
        "ld t6, 15*8(sp)",
        "addi sp, sp, 128",

        "mret",
        handler = sym trap_handler,
    );
}

/// The high-level trap dispatcher
/// RISC-V mcause interpretation:
/// - Interrupt = 1 (top bit)
/// - Exception = 0 (top bit)
extern "C" fn trap_handler(cause: usize, epc: usize) -> usize
{
    let is_interrupt = cause & CAUSE_INTERRUPT_FLAG != 0;
    // Mask out the interrupt bit to get the Exception Code
    let code = cause & !CAUSE_INTERRUPT_FLAG;

    if is_interrupt
    {
        match code
        {
            7 => handle_timer_interrupt(epc), // Machine Timer Interrupt
            11 =>
            {
                handle_external_interrupt();
                epc // Machine External Interrupt (via PLIC)
            }
            _ => epc,
        }
    }
    else
    {
        handle_exception(code, epc)
    }
}

fn handle_timer_interrupt(epc: usize) -> usize
{
    unsafe { timer::schedule_next() }
    Scheduler::schedule(epc)
}

fn handle_external_interrupt()
{
    let irq = unsafe { plic::claim() };

    match irq
    {
        soc::uart::IRQ =>
        {
            if let Some(c) = unsafe { uart::get_char() }
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
        unsafe { plic::complete(irq) };
    }
}

fn handle_exception(code: usize, epc: usize) -> usize
{
    match code
    {
        // Environment Call (ecall) codes for U, S, and M modes.
        8 | 9 | 11 =>
        {
            // Return next instruction address
            epc + 4
        }
        1 => panic!(
            "Instruction Access Fault at {:#x}! (Likely task returned or bad RA)",
            epc
        ),
        2 => panic!("Illegal Instruction at {:#x}!", epc),
        5 => panic!("Load Access Fault at {:#x}!", epc),
        7 => panic!("Store Access Fault at {:#x}!", epc),
        _ => panic!("Unhandled exception: code {}, epc {:#x}", code, epc),
    }
}

/// Enable all the set interrupts with `init`
#[inline]
pub unsafe fn enable()
{
    // mstatus.MIE: Global interrupt enable for Machine Mode
    unsafe { asm!("csrrs x0, mstatus, {0}", in(reg) MIE_FLAG) }
}

/// Initialize Machine-Mode Interrupts
pub unsafe fn init(kernel_sp: usize)
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
    unsafe { asm!("csrw mscratch, {0}", in(reg) kernel_sp) }

    // mie: Enable specific interrupt sources
    unsafe { asm!("csrrs x0, mie, {0}", in(reg) MEIE_FLAG | MTIE_FLAG | MSIE_FLAG) }
}
