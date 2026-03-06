//! Minimal PLIC setup for per-hart interrupt acceptance.

use crate::{
    mmio::{AccessStrategy, Register},
    soc::plic::*,
};

#[inline]
fn get_context(hart_id: usize) -> usize
{
    hart_id * 2 + 1
}

#[inline]
fn threshold_ptr(hart_id: usize) -> Register<u32>
{
    let ctx = get_context(hart_id);
    Register::new(
        (THRESHOLD_BASE + (ctx * 0x1000)) as _,
        AccessStrategy::Direct,
    )
}

#[inline]
pub fn init(hart_id: usize)
{
    threshold_ptr(hart_id).write(0);
}
