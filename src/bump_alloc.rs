use crate::{
    Bytes,
    common::{
        ALLOCATOR_ALREADY_INITIALIZED, ALLOCATOR_UNINITIALIZED, Alloc, Allocations,
        HEAP_END_OVERFLOWED, HEAP_NOT_POWER_TWO, HEAP_SIZE_ZERO, HEAP_START_NULL, Initialization,
        SAllocator, SAllocatorError, align_up,
    },
    std::{
        alloc::Layout,
        cell::UnsafeCell,
        hint::spin_loop,
        ptr::{NonNull, slice_from_raw_parts_mut},
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

pub type BumpAlloc = Alloc<UnsafeCell<Bump>>;

#[derive(Debug)]
pub struct Bump {
    start: usize,
    end: usize,
    next: AtomicUsize,
    allocations: AtomicUsize,
}

unsafe impl SAllocator for UnsafeCell<Bump> {
    fn try_allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, SAllocatorError> {
        debug_assert!(self.is_initialized(), "{ALLOCATOR_UNINITIALIZED}");
        let alloc = unsafe { &*self.get() };

        if layout.size() == 0 {
            alloc.allocations.fetch_add(1, Ordering::AcqRel);

            unsafe {
                return Ok(NonNull::new_unchecked(slice_from_raw_parts_mut(
                    layout.align() as *mut u8,
                    0,
                )));
            }
        }

        loop {
            let next = alloc.next.load(Ordering::Acquire);

            let alloc_start = align_up(next, layout.align());

            let alloc_end = match alloc_start.checked_add(layout.size()) {
                Some(addr) => addr,
                None => return Err(SAllocatorError::InternalOverflow),
            };

            if alloc_end > alloc.end {
                return Err(SAllocatorError::Oom);
            }

            match alloc.next.compare_exchange_weak(
                next,
                alloc_end,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    alloc.allocations.fetch_add(1, Ordering::AcqRel);
                    unsafe {
                        return Ok(NonNull::new_unchecked(slice_from_raw_parts_mut(
                            alloc_start as *mut u8,
                            layout.size(),
                        )));
                    };
                }
                Err(_) => {
                    spin_loop();
                }
            }
        }
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        debug_assert!(self.is_initialized(), "{ALLOCATOR_UNINITIALIZED}");
        let alloc = unsafe { &*self.get() };

        let prev = alloc.allocations.fetch_sub(1, Ordering::AcqRel);

        if prev == 1 {
            alloc.next.store(alloc.start, Ordering::Release);
        }
    }
}

unsafe impl Send for Alloc<UnsafeCell<Bump>> {}
unsafe impl Sync for Alloc<UnsafeCell<Bump>> {}

static INITIALIZED: AtomicBool = AtomicBool::new(false);

impl Initialization for UnsafeCell<Bump> {
    fn new_uninitialized() -> Self {
        UnsafeCell::new(Bump {
            start: 0,
            end: 0,
            next: AtomicUsize::new(0),
            allocations: AtomicUsize::new(0),
        })
    }

    fn is_initialized(&self) -> bool {
        INITIALIZED.load(Ordering::Acquire)
    }

    unsafe fn init(&self, start: usize, size: usize) {
        if INITIALIZED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            panic!("{ALLOCATOR_ALREADY_INITIALIZED}");
        }

        let bump = unsafe { &mut *self.get() };

        assert!(start != 0, "{HEAP_START_NULL}");
        assert!(start.is_power_of_two(), "{HEAP_NOT_POWER_TWO}");
        assert!(size > 0, "{HEAP_SIZE_ZERO}");

        let end = match start.checked_add(size) {
            Some(end) => end,
            None => panic!("{HEAP_END_OVERFLOWED}"),
        };

        bump.start = start;
        bump.end = end;
        bump.next = AtomicUsize::new(start);
        bump.allocations = AtomicUsize::new(0);
    }
}

impl Allocations for UnsafeCell<Bump> {
    fn allocations(&self) -> usize {
        let alloc = unsafe { &*self.get() };
        alloc.allocations.load(Ordering::Acquire)
    }
}

impl Bytes for UnsafeCell<Bump> {
    fn remaining_bytes(&self) -> usize {
        let alloc = unsafe { &*self.get() };

        let next = alloc.next.load(Ordering::Acquire);

        alloc.end.saturating_sub(next)
    }

    fn allocated_bytes(&self) -> usize {
        let alloc = unsafe { &*self.get() };

        let next = alloc.next.load(Ordering::Acquire);

        next.saturating_sub(alloc.start)
    }
}
