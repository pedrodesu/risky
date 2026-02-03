use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicUsize, Ordering},
};

#[repr(usize)]
#[derive(Clone, Copy, PartialEq)]
enum LockState
{
    Empty,
    Initializing,
    Ready,
}

pub struct AtomicLockState(AtomicUsize);

impl AtomicLockState
{
    #[inline]
    pub const fn new(state: LockState) -> Self
    {
        Self(AtomicUsize::new(state as usize))
    }

    #[inline(always)]
    pub fn load(&self, order: Ordering) -> LockState
    {
        // SAFETY: Because the enum is #[repr(usize)] and we control the values, this is
        // safe
        unsafe { core::mem::transmute(self.0.load(order)) }
    }

    #[inline(always)]
    pub fn compare_exchange(
        &self,
        current: LockState,
        new: LockState,
        success: Ordering,
        failure: Ordering,
    ) -> Result<LockState, LockState>
    {
        match self
            .0
            .compare_exchange(current as usize, new as usize, success, failure)
        {
            Ok(v) => Ok(unsafe { core::mem::transmute(v) }),
            Err(v) => Err(unsafe { core::mem::transmute(v) }),
        }
    }

    #[inline(always)]
    pub fn store(&self, val: LockState, order: Ordering)
    {
        self.0.store(val as usize, order);
    }
}

pub struct OnceLock<T>
{
    state: AtomicLockState,
    data: UnsafeCell<Option<T>>,
}

unsafe impl<T: Sync + Send> Sync for OnceLock<T> {}

impl<T> OnceLock<T>
{
    #[inline]
    pub const fn new() -> Self
    {
        Self {
            state: AtomicLockState::new(LockState::Empty),
            data: UnsafeCell::new(None),
        }
    }

    #[inline(always)]
    pub fn get(&self) -> Option<&T>
    {
        // High-performance path: One 'Acquire' load to see if it's Ready
        if self.state.load(Ordering::Acquire) == LockState::Ready
        {
            unsafe { (*self.data.get()).as_ref() }
        }
        else
        {
            None
        }
    }

    pub fn set(&self, value: T) -> Result<(), T>
    {
        // Atomically move from Empty to Initializing.
        // Only one Hart in the entire system can win this race.
        let result = self.state.compare_exchange(
            LockState::Empty,
            LockState::Initializing,
            Ordering::Acquire,
            Ordering::Relaxed,
        );

        if result.is_ok()
        {
            // We won the race. Initialize the data.
            unsafe {
                *self.data.get() = Some(value);
            }
            // Signal to all other Harts that they can now 'get()' the data.
            self.state.store(LockState::Ready, Ordering::Release);
            Ok(())
        }
        else
        {
            // Someone else is already initializing or it's already Ready.
            Err(value)
        }
    }

    /// Wait until the lock is ready (Spinlock style)
    /// Useful for Harts 1-N waiting for Hart 0 to parse the FDT.
    pub fn wait(&self) -> &T
    {
        loop
        {
            if let Some(val) = self.get()
            {
                return val;
            }
            core::hint::spin_loop();
        }
    }
}
