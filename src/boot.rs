//! Kernel runtime and multi-hart initialization logic.

use core::{
    arch::asm,
    sync::atomic::{AtomicU8, Ordering},
};

use ::fdt::Fdt;
use spin::Mutex;

use crate::{
    arch::{CPU_VEC, Cpu},
    demo,
    drivers::uart::{self, UART, Uart},
    fdt, interrupt, logger,
    memory::heap,
    platform::{plic, sbi, timer},
};

#[repr(u8)]
pub enum BootStage
{
    ColdBoot = 0,
    BssInitialized = 1,
    ReadyToWork = 2,
}

#[unsafe(link_section = ".data.boot")]
pub static BOOT_STATUS: AtomicU8 = AtomicU8::new(BootStage::ColdBoot as _);

unsafe extern "C" {
    fn _start();
}

fn set_uart(dev_tree: &Fdt)
{
    let uart_info = fdt::uart::get_info(dev_tree).unwrap();
    UART.call_once(|| Mutex::new(Uart::with_info(uart_info)));
}

fn start_harts()
{
    let cpus = &CPU_VEC.wait();
    let (cpu_zero, rem_cpus) = unsafe { cpus.split_first().unwrap_unchecked() };

    cpu_zero.set();

    for cpu in rem_cpus
    {
        if !sbi::hart_start(cpu.physical_id, _start as *const () as usize, cpu.stack_top)
        {
            log::error!("Failed to start Hart {}", cpu.physical_id);
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

fn cold_boot(hart_id: usize, dev_tree_ptr: *const u8) -> !
{
    let dev_tree = fdt::fdt(dev_tree_ptr).unwrap();

    let hz = fdt::timer::timebase_hz(&dev_tree);
    timer::init(hz);

    set_uart(&dev_tree);
    logger::init();

    log::trace!(
        "Hart {} kmain entry. Device Tree pointer: {:p}",
        hart_id,
        dev_tree_ptr
    );

    heap::init(&dev_tree);
    log::trace!("Heap initialized.");

    Cpu::init_vec(&dev_tree, hart_id);
    BOOT_STATUS.store(BootStage::BssInitialized as _, Ordering::Release);

    start_harts();
    unreachable!();
}

fn secondary_boot(hart_id: usize) -> !
{
    let cpu = CPU_VEC.wait().get(fdt::harts::to_logical(hart_id)).unwrap();
    cpu.set();
    hart_setup();
}

fn hart_setup() -> !
{
    let cpu = Cpu::get();

    log::trace!(
        "Hart {} (Physical {}) is online.",
        cpu.logical_id,
        cpu.physical_id
    );

    log::trace!("Hart {}: Initializing interrupts..", cpu.logical_id);
    interrupt::init(cpu.trap_stack_top);

    plic::init(cpu.physical_id);

    log::trace!("Hart {}: Scheduling next timer interrupt..", cpu.logical_id);
    timer::schedule_next();

    log::trace!("Hart {}: Enabling interrupts..", cpu.logical_id);
    interrupt::enable();

    if cpu.logical_id == 0
    {
        demo::spawn_boot_tasks();
    }

    loop
    {
        uart::drain();

        if !sbi::hart_suspend()
        {
            unsafe { asm!("wfi") }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn kmain(hart_id: usize, opaque: usize) -> !
{
    if BOOT_STATUS.load(Ordering::Acquire) == BootStage::ColdBoot as _
    {
        cold_boot(hart_id, opaque as *const u8)
    }
    else
    {
        secondary_boot(hart_id)
    }
}
