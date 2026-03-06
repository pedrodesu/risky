//! Kernel entrypoint and multi-hart boot orchestration.
//!
//! This file contains the boot path and runtime initialization flow for all
//! harts.

#![no_std]
#![no_main]

use core::{
    arch::{asm, naked_asm},
    panic::PanicInfo,
};

use risky::{BOOT_STATUS, STACK_SIZE, arch::PageAligned, drivers::uart};

#[unsafe(link_section = ".bss.stack")]
static mut BOOT_STACK: PageAligned<STACK_SIZE> = PageAligned([0; _]);

unsafe extern "C" {
    static _bss_start: u8;
    static _bss_end: u8;
}

#[unsafe(link_section = ".text.entry")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub extern "C" fn _start()
{
    naked_asm!(
        // a0 = physical hartid, a1 = FDT pointer (if hart 0) OR heap_stack_top (if hart > 0)
        "mv s0, a0",
        "mv s1, a1",

        // Branching: Master vs Secondary
        "la t0, {boot_status}",
        "lw t1, 0(t0)",
        "bnez t1, 5f",          // If BOOT_STATUS != 0, jump to secondary setup

        // -- Master Hart Setup
        "la t0, {boot_stack}",
        "li t1, {stack_size}",
        "add sp, t0, t1",       // sp = BOOT_STACK + STACK_SIZE

        // Clear BSS (Only on Hart 0)
        "la t0, {bss_start}",
        "la t1, {bss_end}",
        "4:",
        "bge t0, t1, 6f",       // Finished zeroing?
        "sd zero, 0(t0)",       // Wipe 8 bytes
        "addi t0, t0, 8",
        "j 4b",                 // Loop

        "6:",
        "fence iorw, iorw",     // Ensure BSS zeroing is globally visible
        "mv a0, s0",            // Restore for kmain
        "mv a1, s1",            // Restore for kmain
        // a1 still contains FDT pointer, sp points to BOOT_STACK
        "j 3f",

        // -- Secondary Hart Setup
        "5:",
        // a1 contains the `opaque` value (`cpu.stack_top`) from sbi::hart_start
        "mv sp, a1",
        "mv a0, s0",            // Restore hartid
        "li a1, 0",             // Clear a1 so kmain knows this isn't an FDT

        // -- Final Common Setup
        "3:",
        "andi sp, sp, -16",     // Ensure 16-byte alignment for ABI
        "call kmain",
        // If kmain returns (it shouldn't), park the hart safely

        // -- Parking Lot
        "2:",
        "wfi",
        "j 2b",

        boot_status = sym BOOT_STATUS,
        boot_stack  = sym BOOT_STACK,
        stack_size  = const STACK_SIZE,
        bss_start   = sym _bss_start,
        bss_end     = sym _bss_end,
    );
}

#[panic_handler]
fn panic(info: &PanicInfo) -> !
{
    uart::set_direct_mode(true);

    log::error!("\n--- KERNEL PANIC ---");
    log::error!("{}", info);
    log::error!("--------------------");

    loop
    {
        unsafe { asm!("wfi") }
    }
}
