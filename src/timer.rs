//! This module handles the machine-mode timer (MTIMER) part of the Core-Local
//! Interruptor (CLINT). It is used to schedule timer interrupts, which drive
//! the preemptive multitasking of the scheduler.

use crate::soc::clint::*;

pub const INTERVAL: u64 = 100_000;

#[inline]
pub unsafe fn schedule_next()
{
    unsafe {
        // Read the current real-time counter
        let now = MTIME.read_volatile();

        // Schedule the first interrupt
        MTIMECMP.write_volatile(now + INTERVAL);
    }
}
