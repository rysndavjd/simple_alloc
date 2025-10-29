// Portions copyright (c) Philipp Oppermann (https://os.phil-opp.com/)
// Licensed under MIT OR Apache-2.0

use crate::common::{Allocator, AllocatorError, Locked, align_up};
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::{NonNull, null_mut},
};

/// Simple bump allocator using external heap provided via a pointer,
/// initialized at runtime via [`BumpAlloc::init()`].
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

impl BumpAlloc {
    /// Creates a new empty [`BumpAlloc`].
    ///
    /// The allocator must be initialized with [`BumpAlloc::init`] before use.
    pub const fn new() -> Self {
        BumpAlloc {
            start: 0,
            end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initializes the bump allocator with the given heap bounds.
    ///
    /// # Safety
    /// - Must be called only once.
    /// - `heap_start` must be valid memory address (NON-NULL).
    /// - `heap_size` must be greater than 0.
    /// - `heap_start + heap_size` must not overflow.
    /// - The caller must ensure exclusive access to provided memory region for the lifetime of the allocator.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        assert!(heap_start != 0, "Given heap start pointer is NULL");
        assert!(heap_size > 0, "Heap cannot be 0 in size");
        self.start = heap_start;
        self.end = heap_start
            .checked_add(heap_size)
            .expect("Heap end address overflowed");
        self.next = heap_start;
    }

    /// Returns number of allocations currently being handled by the allocator.
    pub fn allocations(&self) -> usize {
        return self.allocations;
    }
}

unsafe impl Allocator for Locked<BumpAlloc> {
    unsafe fn try_allocate(&self, layout: Layout) -> Result<NonNull<u8>, AllocatorError> {
        let mut bump = self.lock();

        let alloc_start = align_up(bump.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return Err(AllocatorError::Overflow),
        };

        if alloc_end > bump.end {
            return Err(AllocatorError::Oom(layout));
        } else {
            bump.next = alloc_end;
            bump.allocations += 1;
            return NonNull::new(alloc_start as *mut u8).ok_or(AllocatorError::Null);
        }
    }

    unsafe fn try_deallocate(
        &self,
        _ptr: NonNull<u8>,
        _layout: Layout,
    ) -> Result<(), AllocatorError> {
        let mut bump = self.lock();
        bump.allocations -= 1;
        if bump.allocations == 0 {
            bump.next = bump.start;
        }

        return Ok(());
    }
}

unsafe impl GlobalAlloc for Locked<BumpAlloc> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            match self.try_allocate(layout) {
                Ok(mut ptr) => return ptr.as_mut(),
                Err(_) => return null_mut(),
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        assert!(!ptr.is_null(), "Given pointer to deallocate is NULL.");
        unsafe {
            self.try_deallocate(NonNull::new_unchecked(ptr), layout)
                .expect("Deallocation failed.")
        }
    }
}
