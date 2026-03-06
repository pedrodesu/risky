//! Bitmap-backed physical page allocator.

use core::{
    ptr::{self, NonNull},
    slice,
};

use crate::arch::PAGE_SIZE;

type BitmapWord = usize;
const BITS_PER_WORD: usize = BitmapWord::BITS as usize;

pub struct BitmapAlloc
{
    bitmap: &'static mut [BitmapWord],
    base_addr: usize,
    total_pages: usize,
}

impl BitmapAlloc
{
    pub unsafe fn new(ptr: NonNull<BitmapWord>, total_pages: usize, base_addr: usize) -> Self
    {
        let word_count = total_pages.div_ceil(BITS_PER_WORD);
        let bitmap = unsafe { slice::from_raw_parts_mut(ptr.as_ptr(), word_count) };

        bitmap.fill(!0);

        Self {
            bitmap,
            base_addr,
            total_pages,
        }
    }

    pub fn mark_free(&mut self, start: usize, size: usize)
    {
        let Some((mut curr, end)) = self.clamped_page_range(start, size, true, false)
        else
        {
            return;
        };

        while curr < end && (curr % BITS_PER_WORD != 0)
        {
            self.set_bit(curr, false);
            curr += 1;
        }

        while curr + BITS_PER_WORD <= end
        {
            self.bitmap[curr / BITS_PER_WORD] = 0;
            curr += BITS_PER_WORD;
        }

        while curr < end
        {
            self.set_bit(curr, false);
            curr += 1;
        }
    }

    pub fn mark_used(&mut self, start: usize, size: usize)
    {
        let Some((start_idx, end_idx)) = self.clamped_page_range(start, size, false, true)
        else
        {
            return;
        };

        for i in start_idx..end_idx
        {
            self.set_bit(i, true);
        }
    }

    pub fn alloc_pages(&mut self, count: usize) -> *mut u8
    {
        if count == 0 || count > self.total_pages
        {
            return ptr::null_mut();
        }

        let mut found = 0;
        let mut start_idx = 0;

        for page_idx in 0..self.total_pages
        {
            if !self.bit_is_set(page_idx)
            {
                if found == 0
                {
                    start_idx = page_idx;
                }

                found += 1;
                if found == count
                {
                    return self.commit_alloc(start_idx, count);
                }
            }
            else
            {
                found = 0;
            }
        }

        ptr::null_mut()
    }

    pub fn free_pages(&mut self, start: *mut u8, count: usize) -> bool
    {
        if count == 0
        {
            return true;
        }

        let addr = start as usize;
        if addr == 0 || addr < self.base_addr
        {
            return false;
        }

        let offset = addr - self.base_addr;
        if offset % PAGE_SIZE != 0
        {
            return false;
        }

        let start_idx = offset / PAGE_SIZE;
        let Some(end_idx) = start_idx.checked_add(count)
        else
        {
            return false;
        };

        if end_idx > self.total_pages
        {
            return false;
        }

        for i in start_idx..end_idx
        {
            self.set_bit(i, false);
        }

        true
    }

    fn commit_alloc(&mut self, start: usize, count: usize) -> *mut u8
    {
        for i in start..(start + count)
        {
            self.set_bit(i, true);
        }

        let Some(offset) = start.checked_mul(PAGE_SIZE)
        else
        {
            return ptr::null_mut();
        };

        let Some(addr) = self.base_addr.checked_add(offset)
        else
        {
            return ptr::null_mut();
        };

        addr as *mut u8
    }

    fn clamped_page_range(
        &self,
        start: usize,
        size: usize,
        round_start_up: bool,
        round_end_up: bool,
    ) -> Option<(usize, usize)>
    {
        if size == 0 || self.total_pages == 0
        {
            return None;
        }

        let end = start.saturating_add(size);
        let start_page = if round_start_up
        {
            Self::page_ceil(start)
        }
        else
        {
            Self::page_floor(start)
        };
        let end_page = if round_end_up
        {
            Self::page_ceil(end)
        }
        else
        {
            Self::page_floor(end)
        };

        let managed_start = self.base_addr / PAGE_SIZE;
        let managed_end = managed_start.saturating_add(self.total_pages);

        let clamped_start = start_page.max(managed_start);
        let clamped_end = end_page.min(managed_end);

        if clamped_start >= clamped_end
        {
            return None;
        }

        Some((clamped_start - managed_start, clamped_end - managed_start))
    }

    #[inline]
    fn page_floor(addr: usize) -> usize
    {
        addr / PAGE_SIZE
    }

    #[inline]
    fn page_ceil(addr: usize) -> usize
    {
        addr.div_ceil(PAGE_SIZE)
    }

    #[inline]
    fn bit_is_set(&self, idx: usize) -> bool
    {
        let word = idx / BITS_PER_WORD;
        let bit = idx % BITS_PER_WORD;
        ((self.bitmap[word] >> bit) & 1) != 0
    }

    fn set_bit(&mut self, idx: usize, val: bool)
    {
        assert!(idx < self.total_pages);
        let word = idx / BITS_PER_WORD;
        let bit = idx % BITS_PER_WORD;
        if val
        {
            self.bitmap[word] |= 1 << bit;
        }
        else
        {
            self.bitmap[word] &= !(1 << bit);
        }
    }
}
