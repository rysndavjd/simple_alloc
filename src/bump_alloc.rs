use crate::{
    Bytes,
    common::{
        ALLOCATOR_ALREADY_INITIALIZED, ALLOCATOR_UNINITIALIZED, Allocations, AllocatorError,
        HEAP_END_OVERFLOWED, HEAP_SIZE_ZERO, HEAP_START_NULL, Initialization, align_up,
        impl_global_alloc,
    },
    std::{
        alloc::Layout,
        cell::UnsafeCell,
        hint::spin_loop,
        ptr::{NonNull, slice_from_raw_parts_mut},
        sync::atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Debug)]
struct Bump {
    start: usize,
    end: usize,
    next: AtomicUsize,
    allocations: AtomicUsize,
}

#[derive(Debug)]
pub struct BumpAlloc(UnsafeCell<Bump>);

impl BumpAlloc {
    /// # Safety
    /// Caller must ensure that allocator has been initialized
    /// else internal state will be invalid.
    unsafe fn get_inner(&self) -> &Bump {
        unsafe { &*(self.0).get() }
    }

    /// # Safety
    /// The caller must guarantee the following:
    ///
    /// - The allocator has been initialized via [`BumpAlloc::init`].
    /// - `layout` satisfies the invariants required by [`Layout::from_size_align`].
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocatorError> {
        debug_assert!(self.is_initialized(), "{ALLOCATOR_UNINITIALIZED}");
        let alloc = unsafe { self.get_inner() };

        if layout.size() == 0 {
            alloc.allocations.fetch_add(1, Ordering::AcqRel);

            // SAFETY:
            // Zero-sized allocations do not require backing memory.
            // We construct a non-null dangling pointer using `layout`
            // alignment as the address. `Layout::from_size_align` guarantees
            // the alignment is non-zero, and the pointer will never be
            // dereferenced because the slice length is zero.
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
                None => return Err(AllocatorError::InternalOverflow),
            };

            if alloc_end > alloc.end {
                return Err(AllocatorError::Oom);
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

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {}

    /// # Safety
    /// Caller must ensure no live references are allocated before calling
    /// this function. Calling reset while allocations are in use will
    /// cause undefined behavior.
    pub unsafe fn reset(&self) {
        let alloc = unsafe { self.get_inner() };

        alloc.next.store(alloc.start, Ordering::Release);
        alloc.allocations.store(0, Ordering::Release);
    }
}

unsafe impl Sync for BumpAlloc {}

impl_global_alloc! {
    BumpAlloc
}

#[cfg(feature = "allocator-api")]
crate::common::impl_allocator_api! {
    BumpAlloc
}

impl Initialization for BumpAlloc {
    fn new_uninitialized() -> Self {
        BumpAlloc(UnsafeCell::new(Bump {
            start: 0,
            end: 0,
            next: AtomicUsize::new(0),
            allocations: AtomicUsize::new(0),
        }))
    }

    fn is_initialized(&self) -> bool {
        let alloc = unsafe { self.get_inner() };

        alloc.start != 0 && alloc.end != 0
    }

    unsafe fn init(&self, start: usize, size: usize) {
        if self.is_initialized() {
            panic!("{ALLOCATOR_ALREADY_INITIALIZED}");
        }

        let bump = unsafe { &mut *(self.0).get() };

        assert!(start != 0, "{HEAP_START_NULL}");
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

impl Allocations for BumpAlloc {
    fn allocations(&self) -> usize {
        let alloc = unsafe { self.get_inner() };
        alloc.allocations.load(Ordering::Acquire)
    }
}

impl Bytes for BumpAlloc {
    fn remaining_bytes(&self) -> usize {
        let alloc = unsafe { self.get_inner() };

        let next = alloc.next.load(Ordering::Acquire);

        alloc.end.saturating_sub(next)
    }

    fn allocated_bytes(&self) -> usize {
        let alloc = unsafe { self.get_inner() };

        let next = alloc.next.load(Ordering::Acquire);

        next.saturating_sub(alloc.start)
    }
}
