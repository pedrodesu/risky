//! Timer and software-interrupt helpers for scheduler preemption.

use spin::Once;

use crate::platform::sbi;

static INTERVAL: Once<u64> = Once::new();

const SIP_SSIP: usize = 1 << 1;

pub fn init(timebase_hz: Option<u64>)
{
    let tick_hz = 100; // 10ms
    INTERVAL.call_once(|| timebase_hz.unwrap_or(10_000_000) / tick_hz);
}

#[cfg(target_arch = "riscv64")]
#[inline]
fn read_time() -> u64
{
    unsafe { csr_read!("time") as u64 }
}

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
        unsafe { csr_clear_i!("sip", SIP_SSIP) }
    }
}

#[inline]
pub fn schedule_next()
{
    let now = read_time();
    sbi::set_timer(now + INTERVAL.wait());
}
