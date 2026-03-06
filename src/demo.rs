//! Demo boot tasks used to validate scheduling and output.

use core::arch::asm;

use crate::task::Task;

#[inline]
fn spin_delay()
{
    for _ in 0..1_000_000
    {
        unsafe { asm!("nop") }
    }
}

#[inline]
pub fn spawn_boot_tasks()
{
    Task::spawn(task_a);
    Task::spawn(task_b);
    Task::spawn(task_c);
}

fn task_a()
{
    loop
    {
        print!("A");
        spin_delay();
    }
}

fn task_b()
{
    loop
    {
        print!("B");
        spin_delay();
    }
}

fn task_c()
{
    println!("C");
}
