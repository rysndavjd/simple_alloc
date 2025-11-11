use crate::common::{ALLOCATOR_UNINITIALIZED, Alloc, BAllocator, BAllocatorError, align_up};
use conquer_once::spin::OnceCell;
use core::{
    alloc::Layout,
    mem::MaybeUninit,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};
#[cfg(feature = "log")]
use log::{debug, error, warn};
use spin::Mutex;

pub type LockedBumpAlloc = Alloc<Mutex<LockedBump>>;
pub type LocklessBumpAlloc = Alloc<OnceCell<LocklessBump>>;
pub type ConstBumpAlloc<const S: usize> = Alloc<ConstBump<S>>;

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
    /// Creates a new empty [`BumpAlloc`].
    ///
    /// The allocator must be initialized with [`BumpAlloc::init`] before use.
    pub const fn new() -> Self {
        LockedBump {
            start: 0,
            end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initializes the bump allocator with the given heap bounds.
    ///
    /// # Safety
    /// - Must be called only once.
    /// - `heap_start` must be valid memory address (NON-NULL).
    /// - `heap_size` must be greater than 0.
    /// - `heap_start + heap_size` must not overflow.
    /// - The caller must ensure exclusive access to provided memory region for the lifetime of the allocator.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
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

    /// Returns number of allocations currently being handled by the allocator.
    pub fn allocations(&self) -> usize {
        return self.allocations;
    }
}

unsafe impl BAllocator for Mutex<LockedBump> {
    unsafe fn init(&self, start: usize, size: usize) {
        unsafe {
            #[cfg(feature = "log")]
            debug!("Initialized locked bump alloc; start: {start:X}, size: {size}");
            self.lock().init(start, size);
        }
    }

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

    fn remaining(&self) -> usize {
        let bump = self.lock();

        return bump.end.checked_sub(bump.next).unwrap_or_default();
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
    /// Creates a new empty [`LocklessBump`].
    ///
    /// The allocator must be initialized with [`LocklessBump::init`] before use.
    pub const fn new() -> Self {
        LocklessBump {
            start: 0,
            end: 0,
            next: AtomicUsize::new(0),
            allocations: AtomicUsize::new(0),
        }
    }

    /// Initializes the bump allocator with the given heap bounds.
    ///
    /// # Safety
    /// - Must be called only once.
    /// - `heap_start` must be valid memory address (NON-NULL).
    /// - `heap_size` must be greater than 0.
    /// - `heap_start + heap_size` must not overflow.
    /// - The caller must ensure exclusive access to provided memory region for the lifetime of the allocator.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        assert!(heap_start != 0, "Given heap start pointer is NULL");
        assert!(heap_size > 0, "Heap cannot be 0 in size");

        self.start = heap_start;
        self.end = heap_start
            .checked_add(heap_size)
            .expect("Heap end address overflowed");
        self.next = AtomicUsize::new(heap_start);
    }

    /// Returns number of allocations currently being handled by the allocator.
    pub fn allocations(&self) -> usize {
        return self.allocations.load(Ordering::SeqCst);
    }
}

unsafe impl BAllocator for OnceCell<LocklessBump> {
    unsafe fn init(&self, start: usize, size: usize) {
        #[cfg(feature = "log")]
        debug!("Initialized lockless bump alloc; start: {start:X}, size: {size}");
        self.init_once(|| {
            let mut bump = LocklessBump::new();
            unsafe {
                bump.init(start, size);
            }
            return bump;
        });
    }

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

    fn remaining(&self) -> usize {
        let alloc = self.get().expect(ALLOCATOR_UNINITIALIZED);

        return alloc
            .end
            .checked_sub(alloc.next.load(Ordering::SeqCst))
            .unwrap_or_default();
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

#[derive(Debug)]
pub struct ConstBump<const S: usize> {
    heap: [MaybeUninit<u8>; S],
    offset: AtomicUsize,
    allocations: AtomicUsize,
}

impl<const S: usize> Default for ConstBump<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const S: usize> ConstBump<S> {
    /// Creates a new [`ConstBump`].
    pub const fn new() -> Self {
        ConstBump {
            heap: [MaybeUninit::<u8>::uninit(); S],
            offset: AtomicUsize::new(0),
            allocations: AtomicUsize::new(0),
        }
    }

    fn heap_start(&self) -> usize {
        return self.heap.as_ptr() as usize;
    }

    fn heap_end(&self) -> usize {
        return (self.heap.as_ptr() as usize) + S;
    }

    fn next(&self) -> usize {
        return self.offset.load(Ordering::SeqCst) + self.heap_start();
    }

    /// Resets the allocator, clearing all previous allocations.
    ///
    /// # Safety
    /// Calling this function while any allocations are still active
    /// may result in undefined behavior. Ensure that no active
    /// allocations exist.
    pub unsafe fn reset(&mut self) {
        self.allocations.store(0, Ordering::SeqCst);
        self.offset.store(0, Ordering::SeqCst);
    }

    /// Returns number of allocations currently being handled by the allocator.
    pub fn allocations(&self) -> usize {
        return self.allocations.load(Ordering::SeqCst);
    }
}

unsafe impl<const S: usize> BAllocator for ConstBump<S> {
    unsafe fn init(&self, _start: usize, _size: usize) {
        #[cfg(feature = "log")]
        warn!("Const bump alloc is already initialized at compile time, so this does nothing.");
    }

    unsafe fn try_allocate(&self, layout: Layout) -> Result<NonNull<u8>, BAllocatorError> {
        let alloc_start = align_up(self.next(), layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return Err(BAllocatorError::Overflowed),
        };

        if alloc_end > self.heap_end() {
            #[cfg(feature = "log")]
            error!("Out of memory");
            return Err(BAllocatorError::Oom(layout));
        } else {
            self.offset.store(
                match alloc_end.checked_sub(self.heap_start()) {
                    Some(end) => end,
                    None => return Err(BAllocatorError::Overflowed),
                },
                Ordering::SeqCst,
            );
            self.allocations.fetch_add(1, Ordering::SeqCst);
            #[cfg(feature = "log")]
            debug!(
                "Allocated object {}; layout: {layout:?}",
                self.allocations.load(Ordering::SeqCst)
            );
            return NonNull::new(alloc_start as *mut u8).ok_or(BAllocatorError::Null);
        }
    }

    unsafe fn try_deallocate(
        &self,
        _ptr: NonNull<u8>,
        _layout: Layout,
    ) -> Result<(), BAllocatorError> {
        let prev = self.allocations.fetch_sub(1, Ordering::AcqRel);
        #[cfg(feature = "log")]
        debug!("Deallocated object {}; layout: {_layout:?}", prev);

        if prev == 1 {
            #[cfg(feature = "log")]
            debug!("All objects deallocated, reseting next pointer to start",);
            self.offset.store(0, Ordering::SeqCst);
        }

        return Ok(());
    }

    fn remaining(&self) -> usize {
        return self.heap_end().checked_sub(self.next()).unwrap_or_default();
    }
}

impl<const S: usize> Alloc<ConstBump<S>> {
    pub const fn new() -> Self {
        Alloc {
            alloc: ConstBump::new(),
        }
    }
}
