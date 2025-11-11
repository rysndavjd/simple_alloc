use crate::common::{
    ALLOCATOR_UNINITIALIZED, Alloc, BAllocator, BAllocatorError, LocklessAlloc, align_up,
};
use conquer_once::spin::OnceCell;
use core::{
    alloc::Layout,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};
#[cfg(feature = "log")]
use log::{debug, error};

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
        assert!(heap_start != 0, "Given heap start pointer is NULL");
        assert!(heap_size > 0, "Heap cannot be 0 in size");

        self.start = heap_start;
        self.end = heap_start
            .checked_add(heap_size)
            .expect("Heap end address overflowed");
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
            #[cfg(feature = "log")]
            error!("Out of memory");
            return Err(BAllocatorError::Oom(layout));
        } else {
            alloc.next.store(alloc_end, Ordering::SeqCst);
            alloc.allocations.fetch_add(1, Ordering::SeqCst);
            #[cfg(feature = "log")]
            debug!(
                "Allocated object {}; layout: {layout:?}",
                alloc.allocations.load(Ordering::SeqCst)
            );
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
        #[cfg(feature = "log")]
        debug!("Deallocated object {}; layout: {_layout:?}", prev);
        if prev == 1 {
            #[cfg(feature = "log")]
            debug!("All objects deallocated, reseting next pointer to start",);
            alloc.next.store(alloc.start, Ordering::SeqCst);
        }

        return Ok(());
    }
}

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

impl LocklessAlloc for Alloc<OnceCell<LocklessBump>> {
    unsafe fn init(&self, start: usize, size: usize) {
        #[cfg(feature = "log")]
        debug!("Initialized lockless bump alloc; start: {start:X}, size: {size}");
        self.alloc.init_once(|| {
            let mut bump = LocklessBump::new();
            unsafe {
                bump.init(start, size);
            }
            return bump;
        });
    }
}
