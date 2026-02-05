//! This module provides a driver for the RISC-V Platform-Level Interrupt
//! Controller (PLIC). The PLIC is responsible for routing external interrupts
//! (like those from UART) to specific CPU cores. This driver handles
//! initialization, interrupt claiming, and completion.

use crate::soc::{Register, plic::*};

/// Get the PLIC context ID for the current Hart in S-mode
/// S-mode context is hart_id * 2 + 1
#[inline]
fn get_context(hart_id: usize) -> usize
{
    hart_id * 2 + 1
}

/// Threshold registers are 0x1000 apart per context
#[inline]
fn threshold_ptr(hart_id: usize) -> Register<u32>
{
    let ctx = get_context(hart_id);
    Register::new((THRESHOLD_BASE + (ctx * 0x1000)) as _)
}
/// Global initialization for the PLIC
#[inline]
pub fn init(hart_id: usize)
{
    // Set threshold to 0 to accept all interrupts with priority > 0
    threshold_ptr(hart_id).write(0);
}
