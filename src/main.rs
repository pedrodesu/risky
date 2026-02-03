//! This is the main entry point for the kernel.
//! It sets up the boot stack for the primary hart (core), parks other harts,
//! and then jumps to `kmain`. The `kmain` function is responsible for
//! initializing all kernel subsystems, starting the scheduler, and spawning
//! initial tasks.

#![no_std]
#![no_main]
#![feature(result_option_map_or_default)]

extern crate alloc;

#[macro_use]
mod arch;

#[macro_use]
mod uart;

mod fdt;
mod heap;
mod interrupt;
mod plic;
mod soc;
mod spin;
mod task;
mod timer;

use alloc::vec::Vec;
use core::{
    arch::{asm, naked_asm},
    panic::PanicInfo,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    spin::Mutex,
    task::{SCHEDULERS, Scheduler, Task},
};

// Ensure the 16-bit alignment of the stacks as per requested by RISC-V ABI
#[repr(align(4096))]
pub struct Aligned<const N: usize>([u8; N]);

const STACK_SIZE: usize = 1024 * 32; // 32KB
const TRAP_STACK_SIZE: usize = 1024 * 8; // 8KB

#[unsafe(link_section = ".bss.stack")]
static mut BOOT_STACK: Aligned<STACK_SIZE> = Aligned([0; _]);

#[unsafe(link_section = ".bss.trap_stack")]
static mut TRAP_STACK: Aligned<TRAP_STACK_SIZE> = Aligned([0; _]);

#[unsafe(link_section = ".text.entry")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
extern "C" fn _start()
{
    naked_asm!(
        // a0 = hartid, a1 = fdt_ptr (passed by the hardware/loader)
        // Save the FDT pointer into a temporary register that survives stack setup
        "mv s1, a1",

        // Calculate this Hart's stack pointer
        // Each hart gets its own slice of BOOT_STACK
        "la t0, {boot_stack}",
        "li t1, {stack_size}",
        "mv t2, a0",            // Get physical hart_id from a0
        "addi t2, t2, 1",       // hart_id + 1
        "mul t1, t1, t2",       // (hart_id + 1) * stack_size
        "add sp, t0, t1",       // sp = boot_stack + offset

        // Enforce 16-byte alignment (RISC-V ABI)
        "andi sp, sp, -16",

        "mv a1, s1",            // Restore FDT pointer to second argument
        // a0 is already the hart_id, so we don't need to move it

        // All harts jump to kmain
        "j kmain",

        boot_stack = sym BOOT_STACK,
        stack_size = const STACK_SIZE,
    );
}

#[unsafe(no_mangle)]
extern "C" fn kmain(hart_id: usize, fdt_ptr: *const u8) -> !
{
    if hart_id == 0
    {
        uart::init();
        heap::init();

        let count = fdt::parse_hart_count(fdt_ptr).unwrap();
        let mut v = Vec::with_capacity(count);
        for _ in 0..count
        {
            v.push(Scheduler::with_task(Task::main()));
        }

        SCHEDULERS.set(v.into_boxed_slice()).ok();
    }
    else
    {
        SCHEDULERS.wait();
    }

    // Move hart_id to tp register. Probably want to put this in its own place
    let logical_id = fdt::physical_to_logical(hart_id);
    unsafe { asm!("mv tp, {}", in(reg) logical_id) };

    plic::init();

    // Calculate the top of the trap stack (highest address)
    let trap_stack_ptr = ((core::ptr::addr_of!(TRAP_STACK) as usize) + TRAP_STACK_SIZE) & !0xF;
    interrupt::init(trap_stack_ptr);

    timer::schedule_next();

    interrupt::enable();

    println!("Hart {} is ready.", hart_id);

    if hart_id == 0
    {
        Task::spawn(task_a);
        Task::spawn(task_b);
        Task::spawn(task_c);
    }

    loop
    {
        unsafe { asm!("wfi") }
    }
}

fn task_a()
{
    loop
    {
        print!("A");

        for _ in 0..1000000
        {
            unsafe { asm!("nop") }
        }
    }
}

fn task_b()
{
    loop
    {
        print!("B");

        for _ in 0..1000000
        {
            unsafe { asm!("nop") }
        }
    }
}

fn task_c()
{
    println!("C");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> !
{
    println!("\n--- KERNEL PANIC ---");
    println!("{}", info);
    println!("--------------------");

    loop
    {
        unsafe { asm!("wfi") }
    }
}
