use crate::std::{
    alloc::{GlobalAlloc, Layout},
    fmt::{Debug, Formatter, Result as FmtResult},
    ptr::{NonNull, copy_nonoverlapping, null_mut},
};

#[inline]
pub fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

pub const HEAP_START_NULL: &str = "Given heap start pointer is NULL";
pub const HEAP_SIZE_ZERO: &str = "Heap cannot be 0 in size";
pub const HEAP_END_OVERFLOWED: &str = "Heap end address overflowed";
pub const HEAP_NOT_POWER_TWO: &str = "Heap not a power of 2";
pub const ALLOCATOR_UNINITIALIZED: &str = "Allocator is not initialized";
pub const ALLOCATOR_ALREADY_INITIALIZED: &str = "Allocator was already initialized";
pub const NULL_PTR: &str = "Null pointer was given";

pub enum SAllocatorError {
    Oom,
    InternalOverflow,
    Alignment(Layout),
}

impl Debug for SAllocatorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            SAllocatorError::Oom => write!(f, "Out of memory"),
            SAllocatorError::InternalOverflow => write!(f, "Overflowed memory allocator internals"),
            SAllocatorError::Alignment(layout) => {
                write!(f, "Unable to satisfy alignment requirement: {layout:?}")
            }
        }
    }
}

pub struct Alloc<A: SAllocator>(A);

impl<A: SAllocator> From<A> for Alloc<A> {
    fn from(a: A) -> Self {
        Alloc(a)
    }
}

/// # Safety
pub unsafe trait SAllocator {
    fn try_allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, SAllocatorError>;

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, SAllocatorError> {
        let ptr = self.try_allocate(layout)?;
        // SAFETY: `alloc` returns a valid memory block
        unsafe { (ptr.as_ptr() as *mut u8).write_bytes(0, ptr.len()) }
        Ok(ptr)
    }

    /// # Safety
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout);

    /// # Safety
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, SAllocatorError> {
        debug_assert!(
            new_layout.size() >= old_layout.size(),
            "`new_layout.size()` must be greater than or equal to `old_layout.size()`"
        );

        let new_ptr = self.try_allocate(new_layout)?;

        // SAFETY: because `new_layout.size()` must be greater than or equal to
        // `old_layout.size()`, both the old and new memory allocation are valid for reads and
        // writes for `old_layout.size()` bytes. Also, because the old allocation wasn't yet
        // deallocated, it cannot overlap `new_ptr`. Thus, the call to `copy_nonoverlapping` is
        // safe. The safety contract for `dealloc` must be upheld by the caller.
        unsafe {
            copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_ptr() as *mut u8, old_layout.size());
            self.deallocate(ptr, old_layout);
        }

        Ok(new_ptr)
    }

    /// # Safety
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, SAllocatorError> {
        debug_assert!(
            new_layout.size() >= old_layout.size(),
            "`new_layout.size()` must be greater than or equal to `old_layout.size()`"
        );

        let new_ptr = self.allocate_zeroed(new_layout)?;

        // SAFETY: because `new_layout.size()` must be greater than or equal to
        // `old_layout.size()`, both the old and new memory allocation are valid for reads and
        // writes for `old_layout.size()` bytes. Also, because the old allocation wasn't yet
        // deallocated, it cannot overlap `new_ptr`. Thus, the call to `copy_nonoverlapping` is
        // safe. The safety contract for `dealloc` must be upheld by the caller.
        unsafe {
            copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_ptr() as *mut u8, old_layout.size());
            self.deallocate(ptr, old_layout);
        }

        Ok(new_ptr)
    }

    /// # Safety
    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, SAllocatorError> {
        debug_assert!(
            new_layout.size() <= old_layout.size(),
            "`new_layout.size()` must be smaller than or equal to `old_layout.size()`"
        );

        let new_ptr = self.try_allocate(new_layout)?;

        // SAFETY: because `new_layout.size()` must be lower than or equal to
        // `old_layout.size()`, both the old and new memory allocation are valid for reads and
        // writes for `new_layout.size()` bytes. Also, because the old allocation wasn't yet
        // deallocated, it cannot overlap `new_ptr`. Thus, the call to `copy_nonoverlapping` is
        // safe. The safety contract for `dealloc` must be upheld by the caller.
        unsafe {
            copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_ptr() as *mut u8, new_layout.size());
            self.deallocate(ptr, old_layout);
        }

        Ok(new_ptr)
    }
}

unsafe impl<A: SAllocator> GlobalAlloc for Alloc<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self.0.try_allocate(layout) {
            Ok(ptr) => ptr.as_ptr() as *mut u8,
            Err(_) => null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        assert!(!ptr.is_null(), "{NULL_PTR}");

        unsafe {
            self.0.deallocate(NonNull::new_unchecked(ptr), layout);
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        match self.0.allocate_zeroed(layout) {
            Ok(ptr) => ptr.as_ptr() as *mut u8,
            Err(_) => null_mut(),
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
        assert!(!ptr.is_null(), "{NULL_PTR}");
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, old_layout.align()) };

        match unsafe {
            self.0
                .grow(NonNull::new_unchecked(ptr), old_layout, new_layout)
        } {
            Ok(ptr) => ptr.as_ptr() as *mut u8,
            Err(_) => null_mut(),
        }
    }
}

#[cfg(feature = "allocator-api")]
unsafe impl<A: SAllocator> crate::std::alloc::Allocator for Alloc<A> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, crate::std::alloc::AllocError> {
        match self.0.try_allocate(layout) {
            Ok(ptr) => Ok(ptr),
            Err(_) => Err(crate::std::alloc::AllocError),
        }
    }

    fn allocate_zeroed(
        &self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, crate::std::alloc::AllocError> {
        match self.0.allocate_zeroed(layout) {
            Ok(ptr) => Ok(ptr),
            Err(_) => Err(crate::std::alloc::AllocError),
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.0.deallocate(ptr, layout) };
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, crate::std::alloc::AllocError> {
        match unsafe { self.0.grow(ptr, old_layout, new_layout) } {
            Ok(ptr) => Ok(ptr),
            Err(_) => Err(crate::std::alloc::AllocError),
        }
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, crate::std::alloc::AllocError> {
        match unsafe { self.0.grow_zeroed(ptr, old_layout, new_layout) } {
            Ok(ptr) => Ok(ptr),
            Err(_) => Err(crate::std::alloc::AllocError),
        }
    }

    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, crate::std::alloc::AllocError> {
        match unsafe { self.0.shrink(ptr, old_layout, new_layout) } {
            Ok(ptr) => Ok(ptr),
            Err(_) => Err(crate::std::alloc::AllocError),
        }
    }
}

pub trait Initialization {
    fn is_initialized(&self) -> bool;

    /// # Safety
    unsafe fn init(&self, start: usize, size: usize);
}

impl<A: SAllocator + Initialization> Initialization for Alloc<A> {
    fn is_initialized(&self) -> bool {
        self.0.is_initialized()
    }

    unsafe fn init(&self, start: usize, size: usize) {
        unsafe { self.0.init(start, size) };
    }
}
