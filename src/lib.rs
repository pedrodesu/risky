//! Kernel crate root.

#![no_std]
#![feature(result_option_map_or_default)]

extern crate alloc;

#[macro_use]
pub mod csr;

#[macro_use]
pub mod drivers;

pub mod arch;
pub mod boot;
pub mod demo;
pub mod fdt;
pub mod interrupt;
pub mod logger;
pub mod memory;
pub mod mmio;
pub mod platform;
pub mod soc;
pub mod task;

pub use boot::{BOOT_STATUS, BootStage};

pub const STACK_SIZE: usize = 1024 * 32; // 32KB
const TRAP_STACK_SIZE: usize = 1024 * 8; // 8KB
