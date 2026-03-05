use conquer_once::spin::OnceCell;

use crate::{
    common::{
        ALLOCATOR_ALREADY_INITIALIZED, ALLOCATOR_UNINITIALIZED, Alloc, AllocInit, Allocations,
        BAllocator, BAllocatorError, HEAP_END_OVERFLOWED, HEAP_SIZE_ZERO, HEAP_START_NULL,
        align_up,
    },
    std::{
        alloc::Layout,
        ptr::{NonNull, slice_from_raw_parts_mut},
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

#[derive(Debug)]
pub struct LocklessBump {
    start: usize,
    end: usize,
    next: AtomicUsize,
    allocations: AtomicUsize,
}

impl LocklessBump {
    const fn empty() -> Self {
        LocklessBump {
            start: 0,
            end: 0,
            next: AtomicUsize::new(0),
            allocations: AtomicUsize::new(0),
        }
    }

    pub fn allocations(&self) -> usize {
        return self.allocations.load(Ordering::SeqCst);
    }
}

unsafe impl BAllocator for OnceCell<LocklessBump> {
    fn try_allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, BAllocatorError> {
        assert!(self.is_initialized(), "{ALLOCATOR_UNINITIALIZED}");

        let alloc = unsafe { self.get_unchecked() };
        let next = alloc.next.load(Ordering::SeqCst);

        let alloc_start = align_up(next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return Err(BAllocatorError::Overflowed),
        };

        if alloc_end > alloc.end {
            return Err(BAllocatorError::Oom);
        } else {
            alloc.next.store(alloc_end, Ordering::SeqCst);
            alloc.allocations.fetch_add(1, Ordering::SeqCst);

            return NonNull::new(slice_from_raw_parts_mut(
                alloc_start as *mut u8,
                layout.size(),
            ))
            .ok_or(BAllocatorError::Null);
        }
    }

    fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        assert!(self.is_initialized(), "{ALLOCATOR_UNINITIALIZED}");

        let alloc = unsafe { self.get_unchecked() };
        let prev = alloc.allocations.fetch_sub(1, Ordering::AcqRel);

        if prev == 1 {
            alloc.next.store(alloc.start, Ordering::SeqCst);
        }
    }
}

unsafe impl Sync for Alloc<OnceCell<LocklessBump>> {}
unsafe impl Send for Alloc<OnceCell<LocklessBump>> {}

impl Alloc<OnceCell<LocklessBump>> {
    pub const fn new() -> Self {
        Alloc {
            alloc: OnceCell::uninit(),
        }
    }
}

static INITIALIZED: AtomicBool = AtomicBool::new(false);

impl AllocInit for OnceCell<LocklessBump> {
    fn is_initialized(&self) -> bool {
        return INITIALIZED.load(Ordering::Acquire);
    }

    unsafe fn init(&self, start: usize, size: usize) {
        if INITIALIZED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            panic!("{ALLOCATOR_ALREADY_INITIALIZED}");
        }

        self.init_once(|| {
            let mut bump = LocklessBump::empty();

            assert!(start != 0, "{HEAP_START_NULL}");
            assert!(size > 0, "{HEAP_SIZE_ZERO}");
            assert!(start + size < usize::MAX, "{HEAP_END_OVERFLOWED}");

            bump.start = start;
            bump.end = start + size;
            bump.next = AtomicUsize::new(start);

            return bump;
        });
    }
}

impl Allocations for OnceCell<LocklessBump> {
    fn allocations(&self) -> usize {
        let alloc = self.get().expect(ALLOCATOR_UNINITIALIZED);
        return alloc.allocations.load(Ordering::Acquire);
    }
}
