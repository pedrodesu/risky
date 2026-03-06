//! Device Tree parsing helpers used during early boot.
//!
//! This module centralizes boot-time hardware discovery from the device tree.

use fdt::{Fdt, FdtError};

#[inline]
pub fn fdt<'a>(fdt_ptr: *const u8) -> Result<Fdt<'a>, FdtError>
{
    unsafe { Fdt::from_ptr(fdt_ptr) }
}

pub mod harts
{
    use alloc::{boxed::Box, vec::Vec};

    use spin::Once;

    use super::*;

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

    pub fn parse_hart_count(dev_tree: &Fdt, boot_hart_id: usize) -> Result<usize, FdtError>
    {
        let mut physical_ids = dev_tree
            .cpus()
            .map(|cpu| cpu.ids().first())
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
}

pub mod uart
{
    use fdt::node::FdtNode;

    use super::*;
    use crate::mmio::IoWidth;

    fn find_node<'a>(dev_tree: &'a Fdt<'a>) -> Option<FdtNode<'a, 'a>>
    {
        let priority_list = [
            "snps,dw-apb-uart", // Modern Desktop/SoC UART
            "ns16550a",         // Classic Generic UART
            "ns16550",          // Legacy Generic UART
        ];

        dev_tree
            .chosen()
            .stdout()
            // If no stdout, check `priority_list` for compatible devices
            .or_else(|| {
                priority_list
                    .iter()
                    .find_map(|&n| dev_tree.find_compatible(&[n]))
            })
    }

    fn parse_node(node: FdtNode) -> Option<(usize, u8, IoWidth)>
    {
        let base = node.reg()?.next()?.starting_address as usize;

        let shift = node
            .property("reg-shift")
            .and_then(|p| Some(u32::from_be_bytes(p.value[0..4].try_into().ok()?) as u8))
            .unwrap_or(0);

        let io_width = node
            .property("reg-io-width")
            .and_then(|p| Some(u32::from_be_bytes(p.value[0..4].try_into().ok()?)))
            .map(|w| if w == 4 { IoWidth::U32 } else { IoWidth::U8 })
            .unwrap_or(IoWidth::U8);

        Some((base, shift, io_width))
    }

    #[inline]
    pub fn get_info(dev_tree: &Fdt) -> Option<(usize, u8, IoWidth)>
    {
        find_node(dev_tree).and_then(parse_node)
    }
}

pub mod mem
{
    use super::*;

    #[inline]
    fn reg_regions<'a>(
        nodes: impl Iterator<Item = fdt::node::FdtNode<'a, 'a>> + 'a,
    ) -> impl Iterator<Item = (usize, usize)> + 'a
    {
        nodes
            .flat_map(|n| n.reg().into_iter().flatten())
            .filter_map(|r| {
                let size = r.size.unwrap_or_default();
                (size != 0).then_some((r.starting_address as usize, size))
            })
    }

    pub fn ram_regions<'a>(dev_tree: &'a Fdt<'a>) -> impl Iterator<Item = (usize, usize)> + 'a
    {
        reg_regions(dev_tree.all_nodes().filter(|n| {
            n.property("device_type")
                .and_then(fdt::node::NodeProperty::as_str)
                .is_some_and(|s| s == "memory")
        }))
    }

    pub fn reserved_regions<'a>(dev_tree: &'a Fdt<'a>)
    -> impl Iterator<Item = (usize, usize)> + 'a
    {
        let fdt_reserved = dev_tree.memory_reservations().filter_map(|r| {
            let size = r.size();
            (size != 0).then_some((r.address() as usize, size))
        });

        let no_map = reg_regions(
            dev_tree
                .all_nodes()
                .filter(|n| n.property("no-map").is_some()),
        );
        fdt_reserved.chain(no_map)
    }
}

pub mod timer
{
    use fdt::Fdt;

    pub fn timebase_hz(dev_tree: &Fdt) -> Option<u64>
    {
        let cpus_node = dev_tree.find_node("/cpus").unwrap(); // This node must exist
        let freq = cpus_node.property("timebase-frequency")?.as_usize()?;
        Some(freq as u64)
    }
}
