//! This module handles the machine-mode timer (MTIMER) part of the Core-Local
//! Interruptor (CLINT). It is used to schedule timer interrupts, which drive
//! the preemptive multitasking of the scheduler.

use crate::sbi;

pub const INTERVAL: u64 = 100_000;

/// Reads the 64-bit TIME register
#[cfg(target_arch = "riscv64")]
#[inline]
fn read_time() -> u64
{
    unsafe { csr_read!("time") as u64 }
}

/// Reads the 64-bit TIME register on a 32-bit architecture.
/// This requires a special sequence to handle potential rollovers of the
/// lower 32 bits during the read.
#[cfg(target_arch = "riscv32")]
fn read_time() -> u64
{
    loop
    {
        let hi = csr_read!("timeh") as u64;
        let lo = csr_read!("time") as u64;
        if hi == csr_read!("timeh")
        {
            return (hi << 32) | lo;
        }
    }
}

pub mod ipi
{
    use super::*;

    #[inline]
    pub fn send(physical_hart_id: usize)
    {
        sbi::send_ipi(1 << physical_hart_id);
    }

    #[inline]
    pub fn clear()
    {
        unsafe { csr_clear_i!("sip", 2) }
    }
}

#[inline]
pub fn schedule_next()
{
    // Read the current real-time counter
    let now = read_time();

    // Schedule the first interval
    sbi::set_timer(now + INTERVAL);
}
