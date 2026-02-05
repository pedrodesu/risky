use core::ptr::NonNull;

/// Register must not be null!
pub struct Register<T>
{
    ptr: NonNull<T>,
}

impl<T> From<*mut T> for Register<T>
{
    #[inline]
    fn from(addr: *mut T) -> Self
    {
        Self {
            ptr: unsafe { NonNull::new_unchecked(addr as _) },
        }
    }
}

impl<T> Register<T>
{
    pub const fn new(addr: *mut T) -> Self
    {
        assert!(!addr.is_null());

        Self {
            ptr: unsafe { NonNull::new_unchecked(addr as _) },
        }
    }

    pub fn read(&self) -> T
    {
        unsafe { self.ptr.read_volatile() }
    }

    pub fn write(&self, value: T)
    {
        unsafe { self.ptr.write_volatile(value) }
    }
}

// Registers for Hart 0 M-Mode
pub mod plic
{
    pub const BASE: usize = 0x0c00_0000;
    pub const THRESHOLD_BASE: usize = BASE + 0x200000;
}
