// Portions copyright (c) Philipp Oppermann (https://os.phil-opp.com/)
// Licensed under MIT OR Apache-2.0

use core::{
    alloc::{GlobalAlloc, Layout},
    mem::MaybeUninit,
    ptr::null_mut,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::common::{Locked, align_up};

pub struct BumpHeap<const S: usize>(pub [MaybeUninit<u8>; S]);

impl<const S: usize> BumpHeap<S> {
    /// Constructs a [`BumpHeap`] with given size `S`
    pub const fn new() -> BumpHeap<S> {
        assert!(S > 0, "Bump heap cannot be zero in size.");
        BumpHeap([MaybeUninit::uninit(); S])
    }
}

impl<const S: usize> Default for BumpHeap<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct BumpAlloc {
    start: usize,
    end: usize,
    next: usize,
    allocations: usize,
}

impl Default for BumpAlloc {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for BumpAlloc {}

impl BumpAlloc {
    pub const fn new() -> Self {
        BumpAlloc {
            start: 0,
            end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initializes the bump allocator with the given heap bounds via a [`BumpHeap`].
    ///
    /// # Safety
    /// - Must be called only once.
    /// - `heap_size` must be greater than 0.
    pub unsafe fn init<const HEAP_SIZE: usize>(&mut self, heap: *mut BumpHeap<HEAP_SIZE>) {
        let start = unsafe { &raw mut (*heap).0 as usize };
        self.start = start;
        self.end = start
            .checked_add(HEAP_SIZE)
            .expect("Heap end address overflowed, Somehow? ¯\\_(ツ)_/¯");
        self.next = start;
    }

    /// Initializes the bump allocator with the given heap bounds.
    ///
    /// # Safety
    /// - Must be called only once.
    /// - `heap_start` must be valid memory address (NON-NULL).
    /// - `heap_size` must be greater than 0.
    /// - `heap_start + heap_size` must not overflow.
    /// - The caller must ensure exclusive access to provided memory region for the lifetime of the allocator.
    pub unsafe fn init_with_ptr(&mut self, heap_start: usize, heap_size: usize) {
        assert!(heap_start != 0, "Given heap start pointer is NULL");
        assert!(heap_size > 0, "Heap cannot be 0 in size");
        self.start = heap_start;
        self.end = heap_start
            .checked_add(heap_size)
            .expect("Heap end address overflowed");
        self.next = heap_start;
    }
}

unsafe impl GlobalAlloc for Locked<BumpAlloc> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump = self.lock();

        let alloc_start = align_up(bump.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return null_mut(),
        };

        if alloc_end > bump.end {
            null_mut()
        } else {
            bump.next = alloc_end;
            bump.allocations += 1;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut bump = self.lock();

        bump.allocations -= 1;
        if bump.allocations == 0 {
            bump.next = bump.start;
        }
    }
}

#[derive(Debug)]
pub struct LocklessBumpAlloc {
    start: usize,
    end: usize,
    next: AtomicUsize,
    allocations: AtomicUsize,
}

impl Default for LocklessBumpAlloc {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for LocklessBumpAlloc {}

impl LocklessBumpAlloc {
    pub const fn new() -> Self {
        LocklessBumpAlloc {
            start: 0,
            end: 0,
            next: AtomicUsize::new(0),
            allocations: AtomicUsize::new(0),
        }
    }

    /// Initializes the lockless bump allocator with the given heap bounds via a [`BumpHeap`].
    ///
    /// # Safety
    /// - Must be called only once.
    /// - `heap_size` must be greater than 0.
    pub unsafe fn init<const HEAP_SIZE: usize>(&mut self, heap: *mut BumpHeap<HEAP_SIZE>) {
        let start = unsafe { &raw mut (*heap).0 as usize };
        self.start = start;
        self.end = start
            .checked_add(HEAP_SIZE)
            .expect("Heap end address overflowed, Somehow? ¯\\_(ツ)_/¯");
        self.next = AtomicUsize::new(start);
    }

    /// Initializes the lockless bump allocator with the given heap bounds.
    ///
    /// # Safety
    /// - Must be called only once.
    /// - `heap_start` must be valid memory address (NON-NULL).
    /// - `heap_size` must be greater than 0.
    /// - `heap_start + heap_size` must not overflow.
    /// - The caller must ensure exclusive access to provided memory region for the lifetime of the allocator.
    pub unsafe fn init_with_ptr(&mut self, heap_start: usize, heap_size: usize) {
        assert!(heap_start != 0, "Given heap start pointer is NULL");
        assert!(heap_size > 0, "Heap cannot be 0 in size");
        self.start = heap_start;
        self.end = heap_start
            .checked_add(heap_size)
            .expect("Heap end address overflowed");
        self.next = AtomicUsize::new(heap_start);
    }
    
    pub fn allocations(&self) -> usize {
        self.allocations.load(Ordering::SeqCst)
    }

    /// Resets the lockless bump allocator state, making the entire heap region available again.
    ///
    /// # Behavior
    ///
    /// - The allocator cursor (`next`) is reset to the start of the heap region.
    /// - The allocation counter (`allocations`) is reset to zero.
    /// - No memory is actually deallocated or zeroed; this is a logical reset only.
    ///
    /// # Safety
    ///
    /// Calling this function is **unsafe** because it invalidates all pointers
    /// previously returned by this allocator. Using any of those pointers after
    /// calling `clear()` results in **undefined behavior**.
    ///
    /// The caller must ensure:
    ///
    /// 1. No other threads are performing allocations or deallocations when running 
    ///    this function.
    /// 2. No live references or pointers from prior allocations are accessed after
    ///    the reset.
    /// 3. If this allocator is shared across threads, external synchronization or
    ///    a global reclamation barrier must be used to guarantee exclusive access.
    pub unsafe fn clear(&mut self) {
        self.allocations.store(0, Ordering::Release);
        self.next.store(self.start, Ordering::Release);
    }
}

unsafe impl GlobalAlloc for LocklessBumpAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let alloc_start = match self.next.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            let aligned = align_up(current, layout.align());
            Some(aligned)
        }) {
            Ok(t) => t,
            Err(_) => return null_mut(),
        };

        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return null_mut(),
        };

        if alloc_end > self.end {
            null_mut()
        } else {
            self.next.store(alloc_end, Ordering::SeqCst);
            self.allocations.fetch_add(1, Ordering::SeqCst);
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

