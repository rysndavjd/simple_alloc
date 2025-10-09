// Portions copyright (c) Philipp Oppermann (https://os.phil-opp.com/)
// Licensed under MIT OR Apache-2.0

use core::{
    alloc::{GlobalAlloc, Layout},
    mem::MaybeUninit,
    ptr::null_mut,
};

use crate::common::{Locked, align_up};

pub struct BumpHeap<const S: usize>(pub [MaybeUninit<u8>; S]);

impl<const S: usize> BumpHeap<S> {
    /// Constructs a [`BumpHeap`] with given size `S`
    pub const fn new() -> BumpHeap<S> {
        BumpHeap([MaybeUninit::uninit(); S])
    }
}

impl<const S: usize> Default for BumpHeap<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct BumpAlloc {
    start: usize,
    end: usize,
    next: usize,
    allocations: usize,
}

impl Default for BumpAlloc {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: unsafe impl Send for BumpAlloc {}

impl BumpAlloc {
    pub const fn new() -> Self {
        BumpAlloc {
            start: 0,
            end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initializes the bump allocator with the given heap bounds via a [`BumpHeap`].
    ///
    /// # Safety
    /// - Must be called only once.
    /// - `heap_size` must be greater than 0.
    pub unsafe fn init<const HEAP_SIZE: usize>(&mut self, heap: *mut BumpHeap<HEAP_SIZE>) {
        assert!(HEAP_SIZE > 0, "Heap cannot be 0 in size");
        let start = unsafe { &raw mut (*heap).0 as usize };
        self.start = start;
        self.end = start
            .checked_add(HEAP_SIZE)
            .expect("Heap end address overflowed, Somehow? ¯\\_(ツ)_/¯");
        self.next = start;
    }

    /// Initializes the bump allocator with the given heap bounds.
    ///
    /// # Safety
    /// - Must be called only once.
    /// - `heap_start` must be valid memory address (NON-NULL).
    /// - `heap_size` must be greater than 0.
    /// - `heap_start + heap_size` must not overflow.
    /// - The caller must ensure exclusive access to provided memory region for the lifetime of the allocator.
    pub unsafe fn init_with_ptr(&mut self, heap_start: usize, heap_size: usize) {
        assert!(heap_start != 0, "Given heap start pointer is NULL");
        assert!(heap_size > 0, "Heap cannot be 0 in size");
        self.start = heap_start;
        self.end = heap_start
            .checked_add(heap_size)
            .expect("Heap end address overflowed");
        self.next = heap_start;
    }
}

unsafe impl GlobalAlloc for Locked<BumpAlloc> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump = self.lock();

        let alloc_start = align_up(bump.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return null_mut(),
        };

        if alloc_end > bump.end {
            null_mut()
        } else {
            bump.next = alloc_end;
            bump.allocations += 1;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut bump = self.lock();

        bump.allocations -= 1;
        if bump.allocations == 0 {
            bump.next = bump.start;
        }
    }
}
