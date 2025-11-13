use core::{
    alloc::Layout,
    mem::MaybeUninit,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

#[cfg(debug_assertions)]
use log::{debug, error};

use crate::common::{Alloc, AllocState, BAllocator, BAllocatorError, OOM, align_up};

#[derive(Debug)]
pub struct ConstBump<const S: usize> {
    heap: [MaybeUninit<u8>; S],
    offset: AtomicUsize,
    allocations: AtomicUsize,
}

impl<const S: usize> Default for ConstBump<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const S: usize> ConstBump<S> {
    const fn new() -> Self {
        ConstBump {
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
}

unsafe impl<const S: usize> BAllocator for ConstBump<S> {
    unsafe fn try_allocate(&self, layout: Layout) -> Result<NonNull<u8>, BAllocatorError> {
        let alloc_start = align_up(self.next(), layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return Err(BAllocatorError::Overflowed),
        };

        if alloc_end > self.heap_end() {
            #[cfg(debug_assertions)]
            error!("{}", OOM);
            return Err(BAllocatorError::Oom(layout));
        } else {
            self.offset.store(
                match alloc_end.checked_sub(self.heap_start()) {
                    Some(end) => end,
                    None => return Err(BAllocatorError::Overflowed),
                },
                Ordering::SeqCst,
            );
            self.allocations.fetch_add(1, Ordering::SeqCst);
            #[cfg(debug_assertions)]
            debug!("Allocated object \"{:X}\"; layout: {layout:?}", alloc_start);
            return NonNull::new(alloc_start as *mut u8).ok_or(BAllocatorError::Null);
        }
    }

    unsafe fn try_deallocate(
        &self,
        _ptr: NonNull<u8>,
        _layout: Layout,
    ) -> Result<(), BAllocatorError> {
        let prev = self.allocations.fetch_sub(1, Ordering::AcqRel);

        if prev == 1 {
            #[cfg(debug_assertions)]
            debug!("All objects deallocated, reseting next pointer to start",);
            self.offset.store(0, Ordering::SeqCst);
        }

        #[cfg(debug_assertions)]
        debug!(
            "Deallocated object \"{:X}\"; layout: {_layout:?}",
            _ptr.as_ptr() as usize
        );
        return Ok(());
    }
}

impl<const S: usize> Alloc<ConstBump<S>> {
    pub const fn new() -> Self {
        Alloc {
            alloc: ConstBump::new(),
        }
    }
}

impl<const S: usize> AllocState for ConstBump<S> {
    fn remaining(&self) -> usize {
        return self.heap_end().checked_sub(self.next()).unwrap_or_default();
    }
    fn allocations(&self) -> usize {
        return self.allocations.load(Ordering::SeqCst);
    }
}
