//! Implements a simple, cooperative, round-robin scheduler.
//!
//! Key components:
//! - `SCHEDULER`: A global, lazily-initialized static instance of the
//!   scheduler.
//! - `Scheduler`: Manages a queue of `waiting_tasks` and tracks the
//!   `current_task`.
//! - `schedule()`: The core scheduling function, called by interrupts to switch
//!   to the next available task. It handles context switching and task state
//!   management.

use alloc::{boxed::Box, collections::VecDeque};
use core::mem;

use super::{Context, Task, TaskKind, TaskState, switch_context};
use crate::spin::{LazyLock, OnceLock};

pub static SCHEDULERS: OnceLock<Box<[Scheduler]>> = OnceLock::new();

pub struct Scheduler
{
    idle_context: Context,
    current_task: Task,
    waiting_tasks: VecDeque<Task>,
}

impl Scheduler
{
    #[inline]
    pub fn with_task(task: Task) -> Self
    {
        Self {
            idle_context: Context::default(),
            current_task: task,
            waiting_tasks: VecDeque::new(),
        }
    }

    #[inline]
    pub fn task_mut(&mut self) -> &mut Task
    {
        &mut self.current_task
    }

    #[inline]
    pub fn add_task(&mut self, task: Task)
    {
        self.waiting_tasks.push_back(task);
    }

    pub fn schedule(interrupted_epc: usize) -> usize
    {
        let scheduler = SCHEDULER.get().unwrap();

        let (old_ctx_ptr, new_ctx_ptr) = {
            let mut scheduler = scheduler.lock();

            // We SHOULD always have at least the main task
            let next_task = match scheduler.waiting_tasks.pop_front()
            {
                Some(task) => task,
                // No other tasks are ready, so just keep the current one.
                None =>
                {
                    // Before returning, we need to unlock the scheduler and
                    // return the interrupted program counter.
                    // This will resume the current task until the next interrupt.
                    return interrupted_epc;
                }
            };
            let mut old_task = mem::replace(&mut scheduler.current_task, next_task);

            let old_ctx_ptr = if
            // The task still isn't over
            old_task.state != TaskState::Dead ||
            // The main task can never end
            old_task.kind == TaskKind::Main
            {
                old_task.context.pc = interrupted_epc;

                let old_ctx = old_task.context.as_mut() as *mut Context;
                scheduler.add_task(old_task);
                old_ctx
            }
            else
            {
                &mut scheduler.idle_context
            };

            scheduler.current_task.state = TaskState::Running;
            let new_ctx_ptr = scheduler.current_task.context.as_ref() as *const Context;

            (old_ctx_ptr, new_ctx_ptr)
        };

        // After this line, we are on a different stack
        // We're switching contexts which means switching stacks. This is why we
        // intentionally drop the mutex before
        unsafe { switch_context(old_ctx_ptr, new_ctx_ptr) };

        let scheduler = scheduler.lock();
        scheduler.current_task.context.pc
    }
}
