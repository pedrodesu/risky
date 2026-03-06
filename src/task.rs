//! Task abstraction, spawning, and lifecycle management.
//!
//! This module defines task types and task lifecycle operations.

mod context;
mod scheduler;

use alloc::boxed::Box;
use core::{
    arch::{asm, naked_asm},
    sync::atomic::{AtomicUsize, Ordering},
};

pub use context::TrapContext;
pub use scheduler::*;

use crate::{
    arch::{CPU_VEC, Cpu},
    interrupt,
    platform::timer,
};

const STACK_SIZE: usize = 1024 * 16; // 16KB

static SPAWN_TICKET: AtomicUsize = AtomicUsize::new(0);

#[derive(PartialEq, Default)]
pub enum TaskState
{
    #[default]
    Ready, // Waiting to be picked
    Running, // Currently on a CPU core
    Dead,    // Finished, waiting to be "reaped" (deleted)
}

pub struct Task
{
    pub context: Box<TrapContext>,
    pub kind: TaskKind,
    pub state: TaskState,
}

#[derive(PartialEq)]
pub enum TaskKind
{
    User
    {
        stack: Box<[u8; STACK_SIZE]>,
    },
    Main,
}

impl Task
{
    #[inline]
    pub fn main() -> Self
    {
        Self {
            context: Box::new(TrapContext::default()),
            kind: TaskKind::Main,
            state: TaskState::default(),
        }
    }

    /// Spawn a task and distribute it across harts in round-robin order.
    pub fn spawn(entry: impl FnOnce() + 'static)
    {
        let n_harts = CPU_VEC.wait().len();
        let ticket = SPAWN_TICKET.fetch_add(1, Ordering::Relaxed);
        let target_hart = ticket % n_harts;
        let target_cpu = Cpu::nth(target_hart);

        let task = Task::from(Box::new(entry) as Box<dyn FnOnce()>);

        interrupt::with_disabled(|| {
            let mut scheduler = target_cpu.scheduler.lock();
            scheduler.add_task(task);
        });

        let local_cpu = Cpu::get();
        if target_hart != local_cpu.logical_id
        {
            timer::ipi::send(target_cpu.physical_id);
        }
    }

    #[unsafe(naked)]
    #[unsafe(no_mangle)]
    pub extern "C" fn trampoline()
    {
        naked_asm!(
            "csrsi sstatus, 2",     // Enable interrupts
            "mv a0, s1",            // `data` argument
            "mv a1, s2",            // `vtable` argument
            "la ra, {exit}",        // Set return address to Task::exit
            "tail {shim}",          // Jump to the shim
            exit = sym Self::exit,
            shim = sym Self::task_entry_shim,
        )
    }

    extern "C" fn task_entry_shim(data: usize, vtable: usize)
    {
        let closure =
            unsafe { Box::from_raw(core::mem::transmute::<_, *mut dyn FnOnce()>((data, vtable))) };

        closure();
    }

    fn exit() -> !
    {
        interrupt::with_disabled(|| {
            let mut scheduler = Cpu::get().scheduler.lock();

            let task = scheduler.task_mut();
            task.state = TaskState::Dead;
        });
        log::info!("Task exited");

        // Trigger a trap to refresh the state immediately
        unsafe { csr_set_i!("sip", 2) } // Raise a supervisor software interrupt

        loop
        {
            unsafe { asm!("wfi") }
        }
    }
}

impl From<Box<dyn FnOnce()>> for Task
{
    fn from(entry_point: Box<dyn FnOnce()>) -> Self
    {
        let mut stack = Box::new([0; _]);

        // Calculate aligned stack top
        let stack_bottom = stack.as_mut_ptr() as usize;
        let stack_top_unaligned = stack_bottom + STACK_SIZE;
        let sp = stack_top_unaligned & !0xF; // Align down to 16 bytes

        // Deconstruct `entry_point` so that we can pass it to `ctx` as two flat
        // pointers
        let entry_ptr = Box::into_raw(entry_point);
        let (data_ptr, vtable_ptr) =
            unsafe { core::mem::transmute::<_, (usize, usize)>(entry_ptr) };

        let ctx = TrapContext {
            ra: Task::trampoline as *const () as usize,
            pc: Task::trampoline as *const () as usize,
            sp,
            s1: data_ptr,
            s2: vtable_ptr,
            ..Default::default()
        };

        Self {
            context: Box::new(ctx),
            kind: TaskKind::User { stack },
            state: Default::default(),
        }
    }
}
