use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::interrupt;

pub struct Mutex<T>
{
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

// This is safe to share between Harts
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T>
{
    #[inline]
    pub const fn new(data: T) -> Self
    {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    #[inline]
    pub unsafe fn data_ptr(&self) -> &T
    {
        // SAFETY: The caller must ensure that the lock is held or that the data is
        // initialised and will never be mutated again.
        unsafe { &*self.data.get() }
    }

    pub fn lock(&self) -> MutexGuard<T>
    {
        let irqs_enabled = (unsafe { csr_read!("mstatus") } & interrupt::MIE_FLAG) != 0;

        // Disable interrupts so we don't get stuck if a trap handler tries to take this
        // same lock on this Hart
        if irqs_enabled
        {
            interrupt::disable();
        }

        while self
            .locked
            .compare_exchange(
                false,
                true,
                Ordering::Acquire, // Ensure we see previous writes after getting the lock
                Ordering::Relaxed,
            )
            .is_err()
        {
            core::hint::spin_loop();
        }

        MutexGuard {
            lock: self,
            interrupt_state: irqs_enabled,
        }
    }
}

pub struct MutexGuard<'a, T>
{
    lock: &'a Mutex<T>,
    interrupt_state: bool, // true if IRQs were enabled before we locked
}

impl<'a, T> Drop for MutexGuard<'a, T>
{
    #[inline]
    fn drop(&mut self)
    {
        self.lock.locked.store(false, Ordering::Release);

        if self.interrupt_state
        {
            interrupt::enable();
        }
    }
}

impl<'a, T> Deref for MutexGuard<'a, T>
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &T
    {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T>
{
    #[inline]
    fn deref_mut(&mut self) -> &mut T
    {
        unsafe { &mut *self.lock.data.get() }
    }
}
