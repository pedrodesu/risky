//! This module provides a driver for the RISC-V Platform-Level Interrupt
//! Controller (PLIC). The PLIC is responsible for routing external interrupts
//! (like those from UART) to specific CPU cores. This driver handles
//! initialization, interrupt claiming, and completion.

use core::arch::asm;

use crate::soc::{Register, plic::*, uart};

/// Priority for IRQ N is at N * 4
/// Priorities are independent of hartid so no context math is needed
#[inline]
const fn priority_ptr(irq: u32) -> Register<u32>
{
    Register::new((PRIORITY_BASE + (irq as usize * 4)) as _)
}

/// Get the PLIC context ID for the current Hart in M-mode
/// M-mode context is hartid * 2
#[inline]
fn get_mmode_context() -> usize
{
    unsafe { csr_read!("mhartid") * 2 }
}

/// Claim registers are 0x1000 apart per context
#[inline]
fn claim_complete_ptr() -> Register<u32>
{
    let ctx = get_mmode_context();
    Register::new((CLAIM_BASE + (ctx * 0x1000)) as _)
}

/// Threshold registers are 0x1000 apart per context
#[inline]
fn threshold_ptr() -> Register<u32>
{
    let ctx = get_mmode_context();
    Register::new((THRESHOLD_BASE + (ctx * 0x1000)) as _)
}

#[inline]
fn enable_ptr(irq: u32) -> Register<u32>
{
    let ctx = get_mmode_context();
    // Each register is 32 bits (4 bytes) wide
    let word_offset = (irq / 32) as usize * 4;
    // Enable registers are 0x80 apart per context
    Register::new((ENABLE_BASE + (ctx * 0x80) + word_offset) as _)
}

/// Global initialization for the PLIC
pub fn init()
{
    // Set priority for UART to 1 to enable it
    // Each IRQ has its own 4-byte priority register
    priority_ptr(uart::IRQ).write(1);

    // Set threshold to 0 to accept all interrupts with priority > 0
    threshold_ptr().write(0);

    // Enable UART IRQ for Hart 0 M-Mode
    // This is a bitmask. IRQ 10 is the 10th bit. Reminder that each register is 32
    // bits wide.
    let ptr = enable_ptr(uart::IRQ);
    let current_mask = ptr.read();
    ptr.write(current_mask | (1 << (uart::IRQ % 32)));
}

/// Claim an interrupt: returns the ID of the highest priority pending interrupt
#[inline]
pub fn claim() -> u32
{
    // Ensure the CPU doesn't try to read from the UART/Device before the PLIC has
    // officially handed us the Interrupt ID
    unsafe { asm!("fence i, r", options(nostack)) }
    claim_complete_ptr().read()
}

/// Complete an interrupt: signals the PLIC that we have handled the IRQ
#[inline]
pub fn complete(irq: u32)
{
    // Ensure our UART/Device processing is written to memory before we tell the
    // PLIC we are done
    unsafe { asm!("fence w, w", options(nostack)) }
    claim_complete_ptr().write(irq);
}
