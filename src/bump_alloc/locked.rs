use core::{
    alloc::Layout,
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
};

#[cfg(debug_assertions)]
use log::{debug, error};
use spin::Mutex;

use crate::common::{
    ALLOCATOR_UNINITIALIZED, Alloc, AllocInit, Allocations, BAllocator, BAllocatorError,
    HEAP_END_OVERFLOWED, HEAP_SIZE_ZERO, HEAP_START_NULL, OOM, align_up,
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
        Self::empty()
    }
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
    fn try_allocate(&self, layout: Layout) -> Result<NonNull<u8>, BAllocatorError> {
        assert!(self.is_initialized(), "{ALLOCATOR_UNINITIALIZED}");

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

    fn try_deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) -> Result<(), BAllocatorError> {
        assert!(self.is_initialized(), "{ALLOCATOR_UNINITIALIZED}");

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

        #[cfg(debug_assertions)]
        debug!("Initialized locked bump alloc; start: {start:#X}, size: {size}");
    }
}

impl Allocations for Mutex<LockedBump> {
    fn allocations(&self) -> usize {
        let alloc = self.lock();
        return alloc.allocations;
    }
}
