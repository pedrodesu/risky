//! This module manages the kernel's dynamic memory allocator.
//! It uses the `talc` allocator, wrapped in a `spin::Mutex` for global-safe
//! access. The heap is initialized at a fixed location after the kernel's
//! `.bss` section.

use core::{
    alloc::{GlobalAlloc, Layout},
    mem,
    num::NonZero,
    ptr::{self, NonNull},
};

use spin::Mutex;
use talc::{ErrOnOom, OomHandler, Span, Talc};

const MAX_ORDER: usize = 11; // 2^11 * 4096 = 8MB max block
const PAGE_SIZE: usize = 4096;

/// PAGE_SHIFT is the log2 of PAGE_SIZE.
/// Shifting an address right by this value converts a byte-address
/// into a zero-based page index (e.g., addr / 4096).
const PAGE_SHIFT: u32 = PAGE_SIZE.trailing_zeros();

struct FreeBlock
{
    prev: Option<NonNull<FreeBlock>>,
    next: Option<NonNull<FreeBlock>>,
}

struct BuddyAllocator
{
    // The start of the managed memory region
    base_addr: NonZero<usize>,
    // Bitmaps for each order.
    // order_bitmaps[0] tracks pairs of 4KB blocks.
    // Each bit represents two buddies.
    order_bitmaps: [*mut u32; MAX_ORDER],
    // Array of linked lists for each order (block size)
    free_lists: [Option<NonNull<FreeBlock>>; MAX_ORDER + 1],
}

impl BuddyAllocator
{
    /// Maps a memory address to its unique bit in the XOR status bitmap for a
    /// specific order.
    ///
    /// The formula (PAGE_SHIFT + order + 1) calculates the magnitude of a
    /// "buddy pair":
    /// - PAGE_SHIFT: Scales from bytes to pages.
    /// - order: Scales to the current block size.
    /// - + 1: Groups two buddies into a single index (effectively dividing by
    ///   2).
    #[inline]
    fn bit_index(&self, addr: NonZero<usize>, order: usize) -> usize
    {
        let offset = addr.get() - self.base_addr.get();

        // We shift right to find which 'pair' of blocks this address belongs to.
        let shift = PAGE_SHIFT + (order as u32) + 1;
        offset >> shift
    }

    /// Toggles the bit representing a pair of buddies and returns the new
    /// state.
    ///
    /// This uses the XOR property to track coalescing:
    /// - Initial state: 0 (Both buddies are in the same state, likely both
    ///   allocated).
    /// - One buddy freed: Bit flips to 1.
    /// - Second buddy freed: Bit flips back to 0.
    ///
    /// If this returns `true`, the buddies can merge and "promote" to the next
    /// order.
    unsafe fn flip_bit(&mut self, addr: NonZero<usize>, order: usize) -> bool
    {
        let idx = self.bit_index(addr, order);

        // Map the linear bit index to a specific 32-bit word and bit position
        let word_idx = idx / 32;
        let bit_idx = idx % 32;

        let bitmap = self.order_bitmaps[order];
        let mask = 1 << bit_idx;

        let old_val = unsafe { bitmap.add(word_idx).read_volatile() };
        let new_val = old_val ^ mask;
        unsafe { bitmap.add(word_idx).write_volatile(new_val) }

        // Return true if the bit is now 0 (meaning both buddies are now free/allocated)
        (new_val & mask) == 0
    }

    #[inline]
    unsafe fn add_to_list(&mut self, order: usize, mut node: NonNull<FreeBlock>)
    {
        unsafe { node.as_mut() }.next = self.free_lists[order].replace(node);
    }

    pub unsafe fn alloc(&mut self, order: usize) -> Option<NonNull<u8>>
    {
        if order > MAX_ORDER
        {
            return None;
        }

        if let Some(block_ptr) = self.free_lists[order]
        {
            unsafe { self.flip_bit(block_ptr.addr(), order) };

            let block_ref = unsafe { block_ptr.read() };
            self.free_lists[order] = block_ref.next;

            return Some(block_ptr.cast());
        }
        else
        {
            // If order is empty, try to split a larger block
            let larger_block = unsafe { self.alloc(order + 1)? };

            let block_size = 1 << (PAGE_SHIFT + (order as u32));
            let buddy = unsafe { larger_block.add(block_size) };

            unsafe { self.add_to_list(order, buddy.cast()) };

            Some(larger_block)
        }
    }

    pub unsafe fn free(&mut self, ptr: NonNull<u8>, order: usize)
    {
        if order >= MAX_ORDER
        {
            unsafe { self.add_to_list(MAX_ORDER, ptr.cast()) };
            return;
        }

        // Flip bit returns true if the buddy is ALSO free
        if unsafe { self.flip_bit(ptr.addr(), order) }
        {
            // Buddy is free! We need to find it and SNIP it from the list.
            let block_size = 1 << (PAGE_SHIFT + (order as u32));
            let buddy_addr = ptr.addr().get() ^ block_size;
            let buddy_ptr = unsafe { NonNull::new_unchecked(buddy_addr as *mut FreeBlock) };

            self.remove_from_list(order, buddy_ptr);

            // Merge them: the new address is the minimum of the two.
            let merged_addr =
                unsafe { NonNull::new_unchecked((ptr.addr().get() & !block_size) as *mut _) };

            unsafe { self.free(merged_addr, order + 1) }
        }
        else
        {
            // Buddy is still allocated, just add this block to the free list.
            unsafe { self.add_to_list(order, ptr.cast()) }
        }
    }
}

pub fn init()
{
    unsafe extern "C" {
        static _end: u8;
    }

    let heap_start = (core::ptr::addr_of!(_end) as usize + 0xF) & !0xF;
}
