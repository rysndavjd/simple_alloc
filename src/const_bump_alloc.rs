use crate::common::{Locked, align_up};
use core::{
    alloc::{GlobalAlloc, Layout},
    mem::MaybeUninit,
    ptr::null_mut,
};

/// Simple bump allocator using internal heap, initialized at compile time.
#[derive(Debug)]
pub struct ConstBumpAlloc<const S: usize> {
    heap: [MaybeUninit<u8>; S],
    offset: usize,
    allocations: usize,
}

impl<const S: usize> ConstBumpAlloc<S> {
    /// Creates a new [`ConstBumpAlloc`].
    pub const fn new() -> Self {
        ConstBumpAlloc {
            heap: [MaybeUninit::<u8>::uninit(); S],
            offset: 0,
            allocations: 0,
        }
    }

    fn heap_start(&self) -> usize {
        return self.heap.as_ptr() as usize;
    }

    fn heap_end(&self) -> usize {
        return (self.heap.as_ptr() as usize) + S;
    }

    fn next(&self) -> usize {
        return self.heap_start() + self.offset;
    }

    /// Returns number of allocations currently being handled by the allocator.
    pub fn allocations(&self) -> usize {
        return self.allocations;
    }
}

unsafe impl<const S: usize> GlobalAlloc for Locked<ConstBumpAlloc<S>> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump = self.lock();

        let alloc_start = align_up(bump.next(), layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return null_mut(),
        };

        if alloc_end > bump.heap_end() {
            null_mut()
        } else {
            bump.offset = match alloc_end.checked_sub(bump.heap_start()) {
                Some(end) => end,
                None => return null_mut(),
            };
            bump.allocations += 1;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut bump = self.lock();

        bump.allocations -= 1;
        if bump.allocations == 0 {
            bump.offset = 0;
        }
    }
}
