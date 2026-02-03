use alloc::{boxed::Box, vec::Vec};

use fdt_rs::{base::*, error::DevTreeError, prelude::*};

use crate::spin::OnceLock;

static HART_ID_MAP: OnceLock<Box<[u32]>> = OnceLock::new();

#[inline]
pub fn physical_to_logical(mhartid: usize) -> usize
{
    let map = HART_ID_MAP.wait();

    map.iter()
        .position(|&phys| phys == mhartid as _)
        .expect("Booting on an unregistered Hart!")
}

pub fn parse_hart_count(fdt_ptr: *const u8) -> Result<usize, DevTreeError>
{
    let fdt = unsafe { DevTree::from_raw_pointer(fdt_ptr) }?;
    let mut phys_ids = Vec::new();

    let mut nodes = fdt.nodes().skip_while(|n| Ok(n.name()? != "cpus"));
    while let Some(node) = nodes.next()?
    {
        let (is_cpu, reg_id) = node.props().fold((false, None), |(cpu, reg), p| {
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
            if let Some(phys_id) = reg_id
            {
                phys_ids.push(phys_id);
            }
        }
        // We've seen CPUs and now we've hit something else. Stop here.
        else if !phys_ids.is_empty()
        {
            break;
        }
    }

    let count = phys_ids.len();

    HART_ID_MAP.set(phys_ids.into_boxed_slice()).unwrap();

    Ok(if count == 0 { 1 } else { count })
}
