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
mod csr;

#[macro_use]
mod uart;

mod arch;
mod fdt;
mod heap;
mod interrupt;
mod plic;
mod sbi;
mod soc;
mod task;
mod timer;

use alloc::alloc::alloc;
use core::{
    alloc::Layout,
    arch::{asm, naked_asm},
    hint,
    panic::PanicInfo,
    sync::atomic::{AtomicU8, Ordering},
};

use spin::Mutex;
use task::{Scheduler, Task};

use crate::arch::{CPU_VEC, Cpu};

// Ensure the page alignment of the stacks
#[repr(align(4096))]
pub struct Aligned<const N: usize>([u8; N]);

const STACK_SIZE: usize = 1024 * 32; // 32KB
const TRAP_STACK_SIZE: usize = 1024 * 8; // 8KB

#[unsafe(link_section = ".bss.stack")]
static mut BOOT_STACK: Aligned<STACK_SIZE> = Aligned([0; _]);

unsafe extern "C" {
    static _bss_start: u8;
    static _bss_end: u8;
}

#[repr(u8)]
enum BootStage
{
    ColdBoot = 0,
    BssInitialized = 1,
    ReadyToWork = 2,
}

#[unsafe(link_section = ".data.boot")]
static BOOT_STATUS: AtomicU8 = AtomicU8::new(BootStage::ColdBoot as _);

#[unsafe(link_section = ".text.entry")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
extern "C" fn _start()
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

fn init_cpu_vec(fdt_ptr: *const u8, boot_hart_id: usize)
{
    let count = fdt::parse_hart_count(fdt_ptr, boot_hart_id).unwrap();
    let cpus = (0..count)
        .map(|i| {
            let layout = Layout::from_size_align(STACK_SIZE, 4096).unwrap(); // Ensure page alignment
            let stack_ptr = unsafe { alloc(layout) as usize };

            let t_layout = Layout::from_size_align(TRAP_STACK_SIZE, 4096).unwrap();
            let trap_ptr = unsafe { alloc(t_layout) as usize };

            Cpu {
                physical_id: fdt::to_physical(i),
                logical_id: i,
                scheduler: Mutex::new(Scheduler::with_task(Task::main())),
                stack_top: stack_ptr + STACK_SIZE,
                trap_stack_top: trap_ptr + STACK_SIZE,
            }
        })
        .collect();

    CPU_VEC.call_once(|| cpus);
}

fn start_harts()
{
    let cpus = &CPU_VEC.wait();
    // Safety: We know we have at least one CPU (CPU zero)
    let (cpu_zero, rem_cpus) = unsafe { cpus.split_first().unwrap_unchecked() };

    cpu_zero.set();

    // Hart 0 is already started.
    for cpu in rem_cpus
    {
        // We pass `stack_to_use` as the `opaque` value. This arrives in `a1` on the
        // other side.
        if !sbi::hart_start(cpu.physical_id, _start as *const () as usize, cpu.stack_top)
        {
            println!("[ERROR] Failed to start Hart {}", cpu.physical_id);
        }
    }

    unsafe {
        asm!(
            "mv sp, {0}",
            "jr {1}",
            in(reg) cpu_zero.stack_top,
            in(reg) hart_setup as *const () as usize,
        )
    }
}

#[unsafe(no_mangle)]
extern "C" fn kmain(hart_id: usize, opaque: usize) -> !
{
    // If we are the Boot Hart, `opaque` is the FDT pointer.
    // If we are Hart 1+, `opaque` is 0 (unused) (set at `_start`)

    if opaque != 0
    {
        let fdt_ptr = opaque as *const u8;
        println!(
            "[TRACE] Hart {} kmain entry. FDT pointer: {:p}",
            hart_id, fdt_ptr
        );

        heap::init();
        println!("[TRACE] Heap initialized.");

        init_cpu_vec(fdt_ptr, hart_id);
        BOOT_STATUS.store(BootStage::BssInitialized as _, Ordering::Release);

        start_harts();
        // We jump to another function in `start_harts`
        unreachable!();
    }
    else
    {
        while BOOT_STATUS.load(Ordering::Acquire) < BootStage::BssInitialized as _
        {
            hint::spin_loop();
        }

        let cpu = CPU_VEC.wait().get(fdt::to_logical(hart_id)).unwrap();
        cpu.set();

        hart_setup();
    }
}

fn hart_setup() -> !
{
    let cpu = Cpu::get();

    println!(
        "[TRACE] Hart {} (Physical {}) is online.",
        cpu.logical_id, cpu.physical_id
    );

    println!("[TRACE] Hart {}: Initializing interrupts..", cpu.logical_id);
    interrupt::init(cpu.trap_stack_top);

    plic::init(cpu.physical_id);

    println!(
        "[TRACE] Hart {}: Scheduling next timer interrupt..",
        cpu.logical_id
    );
    timer::schedule_next();

    println!("[TRACE] Hart {}: Enabling interrupts..", cpu.logical_id);
    interrupt::enable();

    if cpu.logical_id == 0
    {
        Task::spawn(task_a);
        Task::spawn(task_b);
        Task::spawn(task_c);
    }

    loop
    {
        sbi::hart_suspend();
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
