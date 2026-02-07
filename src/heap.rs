//! This module manages the kernel's dynamic memory allocator.
//! It uses the `talc` allocator, wrapped in a `spin::Mutex` for global-safe
//! access. The heap is initialized at a fixed location after the kernel's
//! `.bss` section.

use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::NonNull,
};

use spin::Mutex;
use talc::{ErrOnOom, OomHandler, Span, Talc};

const HEAP_SIZE: usize = 4 * 1024 * 1024; // 4MB

#[global_allocator]
static ALLOCATOR: AllocWrapper<ErrOnOom> = AllocWrapper(Mutex::new(Talc::new(ErrOnOom)));

pub struct AllocWrapper<O: OomHandler>(Mutex<Talc<O>>);

unsafe impl<O: OomHandler> GlobalAlloc for AllocWrapper<O>
{
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8
    {
        unsafe { self.0.lock().malloc(layout) }.map_or_default(NonNull::<u8>::as_ptr)
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout)
    {
        if let Some(ptr) = NonNull::new(ptr)
        {
            unsafe { self.0.lock().free(ptr, layout) }
        }
    }
}

pub fn init()
{
    unsafe extern "C" {
        static _end: u8;
    }

    let heap_start = (core::ptr::addr_of!(_end) as usize + 0xF) & !0xF;
    let heap_range = Span::from_base_size(heap_start as _, HEAP_SIZE);

    unsafe {
        ALLOCATOR
            .0
            .lock()
            .claim(heap_range)
            .expect("Failed to claim heap");
    }
}
