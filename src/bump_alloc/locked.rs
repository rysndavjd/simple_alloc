use core::{alloc::Layout, ptr::NonNull};

#[cfg(debug_assertions)]
use log::{debug, error};
use spin::Mutex;

use crate::common::{
    Alloc, AllocInit, AllocState, BAllocator, BAllocatorError, HEAP_END_OVERFLOWED, HEAP_SIZE_ZERO,
    HEAP_START_NULL, OOM, align_up,
};

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
        debug_assert!(heap_start != 0, "{}", HEAP_START_NULL);
        debug_assert!(heap_size > 0, "{}", HEAP_SIZE_ZERO);
        debug_assert!(
            heap_start + heap_size < usize::MAX,
            "{}",
            HEAP_END_OVERFLOWED
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
            #[cfg(debug_assertions)]
            error!("{}", OOM);
            return Err(BAllocatorError::Oom(Some(layout)));
        } else {
            bump.next = alloc_end;
            bump.allocations += 1;
            #[cfg(debug_assertions)]
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
        let mut bump = self.lock();

        bump.allocations -= 1;
        if bump.allocations == 0 {
            #[cfg(debug_assertions)]
            debug!("All objects deallocated, reseting next pointer to start",);
            bump.next = bump.start;
        }

        #[cfg(debug_assertions)]
        debug!(
            "Deallocated object \"{:X}\"; layout: {_layout:?}",
            _ptr.as_ptr() as usize
        );
        return Ok(());
    }
}

unsafe impl Sync for Alloc<Mutex<LockedBump>> {}
unsafe impl Send for Alloc<Mutex<LockedBump>> {}

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

impl AllocInit for Mutex<LockedBump> {
    unsafe fn init(&self, start: usize, size: usize) {
        unsafe {
            #[cfg(debug_assertions)]
            debug!("Initialized locked bump alloc; start: {start:#X}, size: {size}");
            self.lock().init(start, size);
        }
    }
}

impl AllocState for Mutex<LockedBump> {
    fn remaining(&self) -> usize {
        let alloc = self.lock();
        return alloc.end.checked_sub(alloc.next).unwrap_or_default();
    }
    fn allocations(&self) -> usize {
        let alloc = self.lock();
        return alloc.allocations;
    }
}
