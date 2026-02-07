use alloc::{boxed::Box, vec::Vec};

use fdt::{Fdt, FdtError};
use spin::Once;

static HART_ID_MAP: Once<Box<[usize]>> = Once::new();

#[inline]
pub fn to_logical(physical_hart_id: usize) -> usize
{
    HART_ID_MAP
        .wait()
        .iter()
        .copied()
        .position(|p| p == physical_hart_id)
        .expect("Booting on an unregistered Hart!")
}

#[inline]
pub fn to_physical(logical_hart_id: usize) -> usize
{
    HART_ID_MAP.wait()[logical_hart_id]
}

pub fn parse_hart_count(fdt_ptr: *const u8, boot_hart_id: usize) -> Result<usize, FdtError>
{
    let fdt = unsafe { Fdt::from_ptr(fdt_ptr)? };

    let mut physical_ids = fdt
        .cpus()
        .filter_map(|cpu| Some(cpu.ids().first()))
        .collect::<Vec<_>>();

    if physical_ids.is_empty()
    {
        physical_ids.push(boot_hart_id);
    }
    else if let Some(pos) = physical_ids.iter().position(|&id| id == boot_hart_id)
    {
        // Anchor the current Boot Hart to Logical ID 0
        physical_ids.swap(0, pos);
    }

    let count = physical_ids.len();
    HART_ID_MAP.call_once(|| physical_ids.into_boxed_slice());

    Ok(count)
}
