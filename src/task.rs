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
use core::arch::{asm, naked_asm};

use context::*;
pub use scheduler::*;

const STACK_SIZE: usize = 1024 * 16; // 16KB

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
            "csrsi mstatus, 8",   // Enable interrupts
            "la ra, {exit}",      // Set return address to Task::exit
            "jr s1",              // Jump to entry point
            exit = sym Task::exit,
        )
    }

    #[inline]
    pub fn spawn(entry: *const ())
    {
        let scheduler = SCHEDULER.get().unwrap();
        scheduler.lock().add_task(Task::from(entry));
    }

    fn exit() -> !
    {
        {
            let mut scheduler = SCHEDULER.get().unwrap().lock();

            let task = scheduler.task_mut();
            task.state = TaskState::Dead;
            println!("Task exited");
        }

        // Trigger a trap to refresh the state immediately
        unsafe { asm!("ecall") }

        loop
        {
            unsafe { asm!("wfi") }
        }
    }
}

impl From<*const ()> for Task
{
    fn from(entry_point: *const ()) -> Self
    {
        let mut stack = Box::new([0; _]);

        // Calculate aligned stack top
        let stack_bottom = stack.as_mut_ptr() as usize;
        let stack_top_unaligned = stack_bottom + STACK_SIZE;
        let sp = stack_top_unaligned & !0xF; // Align down to 16 bytes

        let ctx = Context {
            ra: Task::trampoline as *const () as usize,
            pc: Task::trampoline as *const () as usize,
            sp,
            s1: entry_point as usize,
            ..Default::default()
        };

        Self {
            context: Box::new(ctx),
            kind: TaskKind::User { stack },
            state: Default::default(),
        }
    }
}

impl From<()> for Task
{
    #[inline]
    fn from(_: ()) -> Self
    {
        Self {
            context: Box::new(Context::default()),
            kind: TaskKind::Main,
            state: TaskState::default(),
        }
    }
}
