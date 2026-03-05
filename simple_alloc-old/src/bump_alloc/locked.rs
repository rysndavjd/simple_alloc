#[cfg(debug_assertions)]
use spin::Mutex;

use crate::{
    common::{
        ALLOCATOR_UNINITIALIZED, Alloc, AllocInit, Allocations, BAllocator, BAllocatorError,
        HEAP_END_OVERFLOWED, HEAP_SIZE_ZERO, HEAP_START_NULL, align_up,
    },
    std::{
        alloc::Layout,
        ptr::{NonNull, slice_from_raw_parts_mut},
        sync::atomic::{AtomicBool, Ordering},
    },
};

#[derive(Debug)]
pub struct LockedBump {
    start: usize,
    end: usize,
    next: usize,
    allocations: usize,
}

impl LockedBump {
    const fn empty() -> Self {
        LockedBump {
            start: 0,
            end: 0,
            next: 0,
            allocations: 0,
        }
    }

    pub fn allocations(&self) -> usize {
        return self.allocations;
    }
}

unsafe impl BAllocator for Mutex<LockedBump> {
    fn try_allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, BAllocatorError> {
        assert!(self.is_initialized(), "{ALLOCATOR_UNINITIALIZED}");

        let mut bump = self.lock();

        let alloc_start = align_up(bump.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return Err(BAllocatorError::Overflowed),
        };

        if alloc_end > bump.end {
            return Err(BAllocatorError::Oom);
        } else {
            bump.next = alloc_end;
            bump.allocations += 1;
            return NonNull::new(slice_from_raw_parts_mut(
                alloc_start as *mut u8,
                layout.size(),
            ))
            .ok_or(BAllocatorError::Null);
        }
    }

    fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        assert!(self.is_initialized(), "{ALLOCATOR_UNINITIALIZED}");

        let mut bump = self.lock();

        bump.allocations = match bump.allocations.checked_sub(1) {
            Some(t) => t,
            None => panic!("Bump allocations underflowed"),
        };

        if bump.allocations == 0 {
            bump.next = bump.start;
        }
    }
}

unsafe impl Sync for Alloc<Mutex<LockedBump>> {}
unsafe impl Send for Alloc<Mutex<LockedBump>> {}

impl Alloc<Mutex<LockedBump>> {
    pub const fn new() -> Self {
        Alloc {
            alloc: Mutex::new(LockedBump::empty()),
        }
    }
}

static INITIALIZED: AtomicBool = AtomicBool::new(false);

impl AllocInit for Mutex<LockedBump> {
    fn is_initialized(&self) -> bool {
        return INITIALIZED.load(Ordering::Acquire);
    }

    unsafe fn init(&self, start: usize, size: usize) {
        if INITIALIZED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            panic!("Bump Allocator has been initialized already");
        }

        assert!(start != 0, "{}", HEAP_START_NULL);
        assert!(size > 0, "{}", HEAP_SIZE_ZERO);
        assert!(start + size < usize::MAX, "{}", HEAP_END_OVERFLOWED);

        let mut alloc = self.lock();

        alloc.start = start;
        alloc.end = start + size;
        alloc.next = start;
    }
}

impl Allocations for Mutex<LockedBump> {
    fn allocations(&self) -> usize {
        let alloc = self.lock();
        return alloc.allocations;
    }
}
