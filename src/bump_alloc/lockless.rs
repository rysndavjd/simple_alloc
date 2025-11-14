use core::{
    alloc::Layout,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

use conquer_once::spin::OnceCell;
#[cfg(debug_assertions)]
use log::{debug, error};

use crate::common::{
    ALLOCATOR_UNINITIALIZED, Alloc, AllocInit, AllocState, BAllocator, BAllocatorError,
    HEAP_END_OVERFLOWED, HEAP_SIZE_ZERO, HEAP_START_NULL, OOM, align_up,
};

#[derive(Debug)]
pub struct LocklessBump {
    start: usize,
    end: usize,
    next: AtomicUsize,
    allocations: AtomicUsize,
}

impl Default for LocklessBump {
    fn default() -> Self {
        Self::new()
    }
}

impl LocklessBump {
    const fn new() -> Self {
        LocklessBump {
            start: 0,
            end: 0,
            next: AtomicUsize::new(0),
            allocations: AtomicUsize::new(0),
        }
    }

    unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        debug_assert!(heap_start != 0, "{}", HEAP_START_NULL);
        debug_assert!(heap_size > 0, "{}", HEAP_SIZE_ZERO);
        debug_assert!(
            heap_start + heap_size < usize::MAX,
            "{}",
            HEAP_END_OVERFLOWED
        );

        self.start = heap_start;
        self.end = heap_start + heap_size;
        self.next = AtomicUsize::new(heap_start);
    }

    pub fn allocations(&self) -> usize {
        return self.allocations.load(Ordering::SeqCst);
    }
}

unsafe impl BAllocator for OnceCell<LocklessBump> {
    unsafe fn try_allocate(&self, layout: Layout) -> Result<NonNull<u8>, BAllocatorError> {
        let alloc = self.get().expect(ALLOCATOR_UNINITIALIZED);

        let next = alloc.next.load(Ordering::SeqCst);

        let alloc_start = align_up(next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return Err(BAllocatorError::Overflowed),
        };

        if alloc_end > alloc.end {
            #[cfg(debug_assertions)]
            error!("{}", OOM);
            return Err(BAllocatorError::Oom(Some(layout)));
        } else {
            alloc.next.store(alloc_end, Ordering::SeqCst);
            alloc.allocations.fetch_add(1, Ordering::SeqCst);
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
        let alloc = self.get().expect(ALLOCATOR_UNINITIALIZED);
        let prev = alloc.allocations.fetch_sub(1, Ordering::AcqRel);

        if prev == 1 {
            #[cfg(debug_assertions)]
            debug!("All objects deallocated, reseting next pointer to start",);
            alloc.next.store(alloc.start, Ordering::SeqCst);
        }

        #[cfg(debug_assertions)]
        debug!(
            "Deallocated object \"{:X}\"; layout: {_layout:?}",
            _ptr.as_ptr() as usize
        );
        return Ok(());
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

impl Default for Alloc<OnceCell<LocklessBump>> {
    fn default() -> Self {
        Self::new()
    }
}

impl AllocInit for OnceCell<LocklessBump> {
    unsafe fn init(&self, start: usize, size: usize) {
        #[cfg(debug_assertions)]
        debug!("Initialized lockless bump alloc; start: {start:#X}, size: {size}");
        self.init_once(|| {
            let mut bump = LocklessBump::new();
            unsafe {
                bump.init(start, size);
            }
            return bump;
        });
    }
}

impl AllocState for OnceCell<LocklessBump> {
    fn remaining(&self) -> usize {
        let alloc = self.get().expect(ALLOCATOR_UNINITIALIZED);

        return alloc
            .end
            .checked_sub(alloc.next.load(Ordering::SeqCst))
            .unwrap_or_default();
    }
    fn allocations(&self) -> usize {
        let alloc = self.get().expect(ALLOCATOR_UNINITIALIZED);
        return alloc.allocations.load(Ordering::SeqCst);
    }
}
