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

mod heap;
mod interrupt;
mod plic;
mod soc;
mod task;
mod timer;

use core::{
    arch::{asm, naked_asm},
    panic::PanicInfo,
};

use spin::Mutex;

use crate::task::{SCHEDULER, Scheduler, Task};

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
        // Read Hart ID (Core ID) into t0
        "csrr t0, mhartid",

        // Check if Hart ID is 0. If not, jump to park.
        "bnez t0, 2f",

        // Setup Main Stack (Only for Hart 0)
        "la sp, {boot_stack}",
        "li t0, {stack_size}",
        "add sp, sp, t0",
        "andi sp, sp, -16",

        "j kmain",

        // Parking Loop for other cores
        "2:",
        "wfi",
        "j 2b",

        boot_stack = sym BOOT_STACK,
        stack_size = const STACK_SIZE,
    );
}

// main must never return
#[unsafe(no_mangle)]
extern "C" fn kmain() -> !
{
    uart::init();
    println!("UART initialised");

    plic::init();
    println!("PLIC initialised");

    heap::init();
    println!("Allocator initialised");

    SCHEDULER.call_once(|| {
        let main_task = Task::from(());
        let s = Scheduler::with_task(main_task);
        Mutex::new(s)
    });
    println!("Scheduler initialised");

    // Calculate the top of the trap stack (highest address)
    let trap_stack_ptr = ((core::ptr::addr_of!(TRAP_STACK) as usize) + TRAP_STACK_SIZE) & !0xF;
    interrupt::init(trap_stack_ptr);
    println!("Interrupts vector set");

    timer::schedule_next();
    println!("Timer started");

    interrupt::enable();
    println!("Interrupts enabled");

    Task::spawn(task_a);
    Task::spawn(task_b);
    Task::spawn(task_c);

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
