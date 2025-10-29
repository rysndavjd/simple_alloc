use crate::common::{Allocator, AllocatorError, align_up};
use core::{
    alloc::{GlobalAlloc, Layout},
    mem::MaybeUninit,
    ptr::{null_mut, NonNull},
    sync::atomic::{AtomicUsize, Ordering},
};

/// Simple bump allocator using internal heap, initialized at compile time.
#[derive(Debug)]
pub struct ConstBumpAlloc<const S: usize> {
    heap: [MaybeUninit<u8>; S],
    offset: AtomicUsize,
    allocations: AtomicUsize,
}

impl<const S: usize> ConstBumpAlloc<S> {
    /// Creates a new [`ConstBumpAlloc`].
    pub const fn new() -> Self {
        ConstBumpAlloc {
            heap: [MaybeUninit::<u8>::uninit(); S],
            offset: AtomicUsize::new(0),
            allocations: AtomicUsize::new(0),
        }
    }

    fn heap_start(&self) -> usize {
        return self.heap.as_ptr() as usize;
    }

    fn heap_end(&self) -> usize {
        return (self.heap.as_ptr() as usize) + S;
    }

    fn next(&self) -> usize {
        return self.offset.load(Ordering::SeqCst) + self.heap_start();
    }

    /// Resets the allocator, clearing all previous allocations.
    ///
    /// # Safety
    /// Calling this function while any allocations are still active
    /// may result in undefined behavior. Ensure that no active
    /// allocations exist.
    pub unsafe fn reset(&mut self) {
        self.allocations.store(0, Ordering::SeqCst);
        self.offset.store(0, Ordering::SeqCst);
    }

    /// Returns number of allocations currently being handled by the allocator.
    pub fn allocations(&self) -> usize {
        return self.allocations.load(Ordering::SeqCst);
    }
}

unsafe impl<const S: usize> Allocator for ConstBumpAlloc<S> {
    unsafe fn try_allocate(&self, layout: Layout) -> Result<NonNull<u8>, AllocatorError> {
        let alloc_start = align_up(self.next(), layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return Err(AllocatorError::Overflow),
        };

        if alloc_end > self.heap_end() {
            return Err(AllocatorError::Oom(layout));
        } else {
            self.offset.store(match alloc_end.checked_sub(self.heap_start()) {
                Some(end) => end,
                None => return Err(AllocatorError::Overflow),
            }, Ordering::SeqCst);
            self.allocations.fetch_add(1, Ordering::SeqCst);
            return NonNull::new(alloc_start as *mut u8).ok_or(AllocatorError::Null);
        }
    }

    unsafe fn try_deallocate(&self, _ptr: NonNull<u8>, _layout: Layout)
        -> Result<(), AllocatorError> {
        return Ok(());
    }
}

unsafe impl<const S: usize> GlobalAlloc for ConstBumpAlloc<S> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            match self.try_allocate(layout) {
                Ok(mut ptr) => return ptr.as_mut(),
                Err(_) => return null_mut(),
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        assert!(!ptr.is_null(), "Given pointer to deallocate is NULL.");
    }
}
