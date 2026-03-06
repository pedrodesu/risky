//! Kernel global heap allocator bootstrap.

use core::{
    alloc::{GlobalAlloc, Layout},
    mem::size_of,
    ptr::{self, NonNull},
};

use ::fdt::Fdt;
use spin::{Mutex, Once};
use talc::{OomHandler, Span, Talc};

use crate::{
    arch::PAGE_SIZE,
    fdt::mem::{ram_regions, reserved_regions},
    memory::pmm::BitmapAlloc,
};

const INITIAL_HEAP_SIZE: usize = 512 * 1024;
const GROWTH_CHUNK_SIZE: usize = 256 * 1024;
const BITS_PER_WORD: usize = usize::BITS as usize;

pub struct GrowOnOom;

#[global_allocator]
static ALLOCATOR: AllocWrapper<GrowOnOom> = AllocWrapper(Mutex::new(Talc::new(GrowOnOom)));
static PMM: Once<Mutex<BitmapAlloc>> = Once::new();

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

impl OomHandler for GrowOnOom
{
    fn handle_oom(talc: &mut Talc<Self>, layout: Layout) -> Result<(), ()>
    {
        let pmm = PMM.get().ok_or(())?;
        let needed_pages = layout.size().div_ceil(PAGE_SIZE);
        let chunk_pages = GROWTH_CHUNK_SIZE.div_ceil(PAGE_SIZE);
        let pages = needed_pages.max(chunk_pages);

        let start = pmm.lock().alloc_pages(pages);
        let ptr = NonNull::new(start).ok_or(())?;
        let bytes = pages.checked_mul(PAGE_SIZE).ok_or(())?;
        let span = Span::from_base_size(ptr.as_ptr(), bytes);

        match unsafe { talc.claim(span) }
        {
            Ok(_) => Ok(()),
            Err(_) =>
            {
                let _ = pmm.lock().free_pages(start, pages);
                Err(())
            }
        }
    }
}

pub fn pmm() -> &'static Mutex<BitmapAlloc>
{
    PMM.wait()
}

pub fn init(dev_tree: &Fdt)
{
    unsafe extern "C" {
        static _end: u8;
    }

    let Some((ram_start, ram_end)) = ({
        let (min, max) =
            ram_regions(dev_tree).fold((usize::MAX, 0), |(min, max), (start, size)| {
                let end = start.saturating_add(size);
                (min.min(start), max.max(end))
            });

        (min < max).then_some((min, max))
    })
    else
    {
        panic!("No RAM detected.");
    };

    let kernel_end = ptr::addr_of!(_end) as usize;

    let (managed_start_page, managed_end_page) = {
        let (free_mem_start, free_mem_end) = (ram_start.max(kernel_end), ram_end);
        (free_mem_start.div_ceil(PAGE_SIZE), free_mem_end / PAGE_SIZE)
    };

    if managed_start_page >= managed_end_page
    {
        panic!("Not enough RAM remains after kernel image.")
    }

    let managed_start = managed_start_page
        .checked_mul(PAGE_SIZE)
        .expect("Managed start overflow.");
    let total_pages = managed_end_page - managed_start_page;

    let bitmap_pages = {
        let bitmap_words = total_pages.div_ceil(BITS_PER_WORD);
        let bitmap_bytes = bitmap_words
            .checked_mul(size_of::<usize>())
            .expect("Bitmap size overflow.");

        bitmap_bytes.div_ceil(PAGE_SIZE)
    };

    if bitmap_pages >= total_pages
    {
        panic!("Not enough RAM for PMM.");
    }

    let bitmap_base = managed_start;
    let bitmap_bytes_aligned = bitmap_pages
        .checked_mul(PAGE_SIZE)
        .expect("Bitmap page bytes overflow.");
    let pmm_base = managed_start
        .checked_add(bitmap_bytes_aligned)
        .expect("PMM base overflow.");
    let pmm_pages = total_pages - bitmap_pages;

    let bitmap_ptr =
        NonNull::new(bitmap_base as *mut usize).expect("PMM bitmap base cannot be a null pointer.");

    let mut pmm = unsafe { BitmapAlloc::new(bitmap_ptr, pmm_pages, pmm_base) };

    ram_regions(dev_tree).for_each(|(start, size)| pmm.mark_free(start, size));
    reserved_regions(dev_tree).for_each(|(start, size)| pmm.mark_used(start, size));

    let kernel_reserved_start = ram_start.max(pmm_base);
    if kernel_end > kernel_reserved_start
    {
        pmm.mark_used(kernel_reserved_start, kernel_end - kernel_reserved_start);
    }
    pmm.mark_used(bitmap_base, bitmap_bytes_aligned);

    let heap_pages = INITIAL_HEAP_SIZE.div_ceil(PAGE_SIZE);
    let heap_start =
        NonNull::new(pmm.alloc_pages(heap_pages)).expect("Failed to allocate heap pages.");

    PMM.call_once(|| Mutex::new(pmm));

    let heap_bytes = heap_pages
        .checked_mul(PAGE_SIZE)
        .expect("Heap byte size overflow.");
    let heap_range = Span::from_base_size(heap_start.as_ptr(), heap_bytes);

    unsafe {
        ALLOCATOR
            .0
            .lock()
            .claim(heap_range)
            .expect("Failed to claim heap");
    }
}
