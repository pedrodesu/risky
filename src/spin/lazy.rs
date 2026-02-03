use core::{
    cell::OnceCell,
    ops::Deref,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::spin::mutex::Mutex;

pub struct LazyLock<T, F>
{
    cell: Mutex<OnceCell<T>>,
    init: F,
    initialized: AtomicBool,
}

impl<T, F: FnOnce() -> T> LazyLock<T, F>
{
    #[inline]
    pub const fn new(init: F) -> Self
    {
        Self {
            cell: Mutex::new(OnceCell::new()),
            init,
            initialized: AtomicBool::new(false),
        }
    }
}

unsafe impl<T: Sync, F: Send> Sync for LazyLock<T, F> {}

impl<T, F: Fn() -> T> Deref for LazyLock<T, F>
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &T
    {
        if self.initialized.load(Ordering::Acquire)
        {
            // SAFETY: initialized is true, so OnceCell is never written to again, making it
            // safe to return a shared reference
            unsafe { self.cell.data_ptr().get().unwrap_unchecked() }
        }
        else
        {
            self.get_or_init_slow()
        }
    }
}

impl<T, F: Fn() -> T> LazyLock<T, F>
{
    #[cold] // Will rarely be called
    fn get_or_init_slow(&self) -> &T
    {
        let guard = self.cell.lock();

        let val = guard.get_or_init(|| (self.init)());

        self.initialized.store(true, Ordering::Release);

        unsafe { &*(val as *const T) }
    }
}
