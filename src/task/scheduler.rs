//! Per-hart round-robin scheduler and context-switch policy.
//!
//! This module manages runnable tasks and scheduling decisions per hart.

use alloc::collections::VecDeque;
use core::mem;

use super::{Task, TaskKind, TaskState, TrapContext};

pub struct Scheduler
{
    current_task: Task,
    waiting_tasks: VecDeque<Task>,
}

impl Scheduler
{
    #[inline]
    pub fn with_task(task: Task) -> Self
    {
        Self {
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

    pub fn schedule(&mut self, frame: &mut TrapContext)
    {
        // Persist interrupted task state unless it has already terminated.
        if self.current_task.state != TaskState::Dead
        {
            self.current_task.state = TaskState::Ready;
            *self.current_task.context = *frame;
        }

        // We should always have at least the main task as a runnable fallback.
        let next_task = match self.waiting_tasks.pop_front()
        {
            Some(task) => task,
            // No other tasks are ready, keep running the current one.
            None =>
            {
                if self.current_task.state == TaskState::Dead
                {
                    panic!("No runnable tasks available");
                }
                self.current_task.state = TaskState::Running;
                return;
            }
        };

        let old_task = mem::replace(&mut self.current_task, next_task);

        if old_task.state != TaskState::Dead || old_task.kind == TaskKind::Main
        {
            self.add_task(old_task);
        }

        self.current_task.state = TaskState::Running;
        *frame = *self.current_task.context;
    }
}
