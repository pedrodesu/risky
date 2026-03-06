//! Generic MMIO register access primitives.

use core::ptr::NonNull;

#[derive(Clone, Copy)]
pub enum IoWidth
{
    U8,
    U32,
}

#[derive(Clone, Copy)]
pub enum AccessStrategy
{
    Direct,
    Shifted
    {
        width: IoWidth,
    },
}

pub trait RegisterValue: Copy
{
    fn to_u32(self) -> u32;
    fn from_u32(val: u32) -> Self;
}

impl RegisterValue for u8
{
    #[inline(always)]
    fn to_u32(self) -> u32
    {
        self as u32
    }

    #[inline(always)]
    fn from_u32(val: u32) -> Self
    {
        val as u8
    }
}

impl RegisterValue for u32
{
    #[inline(always)]
    fn to_u32(self) -> u32
    {
        self
    }

    #[inline(always)]
    fn from_u32(val: u32) -> Self
    {
        val
    }
}

pub struct Register<T>
{
    ptr: NonNull<T>,
    strategy: AccessStrategy,
}

impl<T: RegisterValue> Register<T>
{
    pub const fn new(ptr: *mut T, strategy: AccessStrategy) -> Self
    {
        assert!(!ptr.is_null());

        Self {
            ptr: unsafe { NonNull::new_unchecked(ptr as _) },
            strategy,
        }
    }

    pub fn read(&self) -> T
    {
        unsafe {
            match self.strategy
            {
                AccessStrategy::Direct | AccessStrategy::Shifted { width: IoWidth::U8 } =>
                {
                    self.ptr.read_volatile()
                }
                AccessStrategy::Shifted {
                    width: IoWidth::U32,
                } => T::from_u32((self.ptr.as_ptr() as *const u32).read_volatile()),
            }
        }
    }

    pub fn write(&self, value: T)
    {
        unsafe {
            match self.strategy
            {
                AccessStrategy::Direct | AccessStrategy::Shifted { width: IoWidth::U8 } =>
                {
                    self.ptr.write_volatile(value);
                }
                AccessStrategy::Shifted {
                    width: IoWidth::U32,
                } =>
                {
                    (self.ptr.as_ptr() as *mut u32).write_volatile(value.to_u32());
                }
            }
        }
    }
}
