use crate::common::{Alloc, BAllocator, BAllocatorError, LockedAlloc, align_up};
use core::{alloc::Layout, ptr::NonNull};
#[cfg(feature = "log")]
use log::{debug, error};
use spin::Mutex;

#[derive(Debug)]
pub struct LockedBump {
    start: usize,
    end: usize,
    next: usize,
    allocations: usize,
}

impl Default for LockedBump {
    fn default() -> Self {
        Self::new()
    }
}

impl LockedBump {
    const fn new() -> Self {
        LockedBump {
            start: 0,
            end: 0,
            next: 0,
            allocations: 0,
        }
    }

    unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        debug_assert!(heap_start != 0, "Given heap start pointer is NULL");
        debug_assert!(heap_size > 0, "Heap cannot be 0 in size");
        debug_assert!(
            heap_start + heap_size < usize::MAX,
            "Heap end address overflowed"
        );

        self.start = heap_start;
        self.end = heap_start + heap_size;
        self.next = heap_start;
    }

    pub fn allocations(&self) -> usize {
        return self.allocations;
    }
}

unsafe impl BAllocator for Mutex<LockedBump> {
    unsafe fn try_allocate(&self, layout: Layout) -> Result<NonNull<u8>, BAllocatorError> {
        let mut bump = self.lock();

        let alloc_start = align_up(bump.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return Err(BAllocatorError::Overflowed),
        };

        if alloc_end > bump.end {
            #[cfg(feature = "log")]
            error!("Out of memory");
            return Err(BAllocatorError::Oom(layout));
        } else {
            bump.next = alloc_end;
            bump.allocations += 1;
            #[cfg(feature = "log")]
            debug!("Allocated object {}; layout: {layout:?}", bump.allocations);
            return NonNull::new(alloc_start as *mut u8).ok_or(BAllocatorError::Null);
        }
    }

    unsafe fn try_deallocate(
        &self,
        _ptr: NonNull<u8>,
        _layout: Layout,
    ) -> Result<(), BAllocatorError> {
        let mut bump = self.lock();
        #[cfg(feature = "log")]
        debug!(
            "Deallocated object {}; layout: {_layout:?}",
            bump.allocations
        );
        bump.allocations -= 1;
        if bump.allocations == 0 {
            #[cfg(feature = "log")]
            debug!("All objects deallocated, reseting next pointer to start",);
            bump.next = bump.start;
        }

        return Ok(());
    }
}

impl Alloc<Mutex<LockedBump>> {
    pub const fn new() -> Self {
        Alloc {
            alloc: Mutex::new(LockedBump::new()),
        }
    }
}

impl Default for Alloc<Mutex<LockedBump>> {
    fn default() -> Self {
        Self::new()
    }
}

impl LockedAlloc for Mutex<LockedBump> {
    unsafe fn init(&self, start: usize, size: usize) {
        unsafe {
            #[cfg(feature = "log")]
            debug!("Initialized locked bump alloc; start: {start:X}, size: {size}");
            self.lock().init(start, size);
        }
    }
}

impl LockedAlloc for Alloc<Mutex<LockedBump>> {
    unsafe fn init(&self, start: usize, size: usize) {
        unsafe { self.alloc.init(start, size) };
    }
}
