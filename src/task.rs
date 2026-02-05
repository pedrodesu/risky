//! This module defines the core task management structures and logic.
//!
//! It includes:
//! - `Task`, `TaskKind`, and `TaskState` for representing and managing tasks.
//! - A `trampoline` function to safely start tasks and ensure they call
//!   `Task::exit`.
//! - `Task::spawn` for creating new user-space tasks.
//! - `Task::exit` for gracefully terminating tasks and triggering a reschedule.

mod context;
mod scheduler;

use alloc::boxed::Box;
use core::{
    arch::{asm, naked_asm},
    sync::atomic::{AtomicUsize, Ordering},
};

use context::*;
pub use scheduler::*;

use crate::{
    arch::{CPU_VEC, Cpu},
    timer,
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
    pub context: Box<Context>,
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

    #[inline]
    pub fn main() -> Self
    {
        Self {
            context: Box::new(Context::default()),
            kind: TaskKind::Main,
            state: TaskState::default(),
        }
    }

    pub fn spawn(entry: impl FnOnce() + 'static)
    {
        let cpus = CPU_VEC.wait();
        let n_harts = cpus.len();

        let ticket = SPAWN_TICKET.fetch_add(1, Ordering::Relaxed);
        let target_hart = ticket % n_harts;

        let boxed: Box<dyn FnOnce()> = Box::new(entry);

        {
            let mut scheduler = cpus[target_hart].scheduler.lock();
            scheduler.add_task(Task::from(boxed));

            // Drop the lock before sending the IPI to avoid a race where
            // the target wakes up and tries to lock the scheduler while we
            // still hold it.
        }

        let cpu = Cpu::get();
        // Wake the target only if it's not us
        if target_hart != cpu.logical_id
        {
            timer::ipi::send(cpus[target_hart].physical_id);
        }
    }

    fn exit() -> !
    {
        {
            let mut scheduler = Cpu::get().scheduler.lock();

            let task = scheduler.task_mut();
            task.state = TaskState::Dead;
            println!("Task exited");
        }

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

        let ctx = Context {
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
