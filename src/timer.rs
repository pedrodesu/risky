//! This module handles the machine-mode timer (MTIMER) part of the Core-Local
//! Interruptor (CLINT). It is used to schedule timer interrupts, which drive
//! the preemptive multitasking of the scheduler.

use crate::{arch, soc::clint::*};

pub const INTERVAL: u64 = 100_000;

/// Reads the 64-bit MTIME register.
#[cfg(target_arch = "riscv64")]
fn read_time() -> u64
{
    MTIME.read()
}

/// Reads the 64-bit MTIME register on a 32-bit architecture.
/// This requires a special sequence to handle potential rollovers of the
/// lower 32 bits during the read.
#[cfg(target_arch = "riscv32")]
fn read_time() -> u64
{
    loop
    {
        let high = MTIMEH.read();
        let low = MTIME.read();
        // If the high bits haven't changed, we have a consistent 64-bit value
        if high == MTIMEH.read()
        {
            return ((high as u64) << 32) | (low as u64);
        }
    }
}

#[inline]
pub fn schedule_next()
{
    // Read the current real-time counter
    let now = read_time();

    // Schedule the first interrupt
    mtimecmp(arch::hart_id()).write(now + INTERVAL)
}
