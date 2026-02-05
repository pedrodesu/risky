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

use alloc::collections::VecDeque;
use core::mem;

use super::{Context, Task, TaskKind, TaskState, switch_context};

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

    pub fn schedule(&mut self, interrupted_epc: usize) -> usize
    {
        println!("[TRACE] In scheduler::schedule");

        // We SHOULD always have at least the main task
        let next_task = match self.waiting_tasks.pop_front()
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
        let mut old_task = mem::replace(&mut self.current_task, next_task);

        let old_ctx_ptr = if
        // The task still isn't over
        old_task.state != TaskState::Dead ||
            // The main task can never end
            old_task.kind == TaskKind::Main
        {
            old_task.context.pc = interrupted_epc;

            let old_ctx = old_task.context.as_mut() as *mut Context;
            self.add_task(old_task);
            old_ctx
        }
        else
        {
            let old_ctx = &mut self.idle_context;
            drop(old_task);
            old_ctx
        };

        self.current_task.state = TaskState::Running;
        let new_ctx_ptr = self.current_task.context.as_ref() as *const Context;
        let new_pc = self.current_task.context.pc;

        // After this line, we are on a different stack
        // We're switching contexts which means switching stacks. This is why we
        // intentionally drop the mutex before
        unsafe { switch_context(old_ctx_ptr, new_ctx_ptr) };

        new_pc
    }
}
