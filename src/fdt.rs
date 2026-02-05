use alloc::{boxed::Box, vec::Vec};

use fdt_rs::{base::*, error::DevTreeError, prelude::*};
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

pub fn parse_hart_count(fdt_ptr: *const u8) -> Result<usize, DevTreeError>
{
    let fdt = unsafe { DevTree::from_raw_pointer(fdt_ptr) }?;
    let mut physical_ids = Vec::new();

    let mut nodes = fdt.nodes().skip_while(|n| Ok(n.name()? != "cpus"));
    while let Some(node) = nodes.next()?
    {
        let (is_cpu, reg) = node.props().fold((false, None), |(cpu, reg), p| {
            Ok(match p.name()?
            {
                "device_type" if p.str()? == "cpu" => (true, reg),
                // reg is usually a u32 on RISC-V 32/64
                "reg" => (cpu, Some(p.u32(0)?)),
                _ => (cpu, reg),
            })
        })?;

        if is_cpu
        {
            if let Some(reg) = reg
            {
                physical_ids.push(reg as _);
            }
        }
        // We've seen CPUs and now we've hit something else. Stop here.
        else if !physical_ids.is_empty()
        {
            break;
        }
    }

    if physical_ids.is_empty()
    {
        // We didn't find any CPUs in the device tree,
        // but we know at least one exists because we are running on it.

        // Assume Hart 0 as a sane default if FDT is broken.
        physical_ids.push(0);
    }

    let count = physical_ids.len();

    HART_ID_MAP.call_once(|| physical_ids.into_boxed_slice());

    Ok(count)
}
