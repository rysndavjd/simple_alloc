use crate::std::{
    alloc::Layout,
    fmt::{Debug, Formatter, Result as FmtResult},
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

/// A list specifying allocator errors.
/// This may later grow to include additional errors or reduced.
#[non_exhaustive]
pub enum AllocatorError {
    /// Insufficient memory to allocate given object.
    Oom,
    /// Unable to satisfy alignment requirements for given [`layout`]
    Alignment(Layout),
    /// Internal state of memory allocator has overflowed.
    InternalOverflow,
}

impl Debug for AllocatorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            AllocatorError::Oom => write!(f, "Out of memory"),
            AllocatorError::Alignment(layout) => {
                write!(f, "Unable to satisfy alignment requirement: {layout:?}")
            }
            AllocatorError::InternalOverflow => write!(f, "Overflowed memory allocator internals"),
        }
    }
}

/// Generates `GlobalAlloc` implementation for given object.
macro_rules! impl_global_alloc {
    ($t:ty) => {
        unsafe impl crate::std::alloc::GlobalAlloc for $t {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                match self.allocate(layout) {
                    Ok(ptr) => ptr.as_ptr() as *mut u8,
                    Err(_) => crate::std::ptr::null_mut(),
                }
            }

            unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
                assert!(!ptr.is_null(), "{}", crate::common::NULL_PTR);
                unsafe { self.deallocate(NonNull::new_unchecked(ptr), layout) }
            }
        }
    };
}

pub(crate) use impl_global_alloc;

#[cfg(feature = "allocator-api")]
/// Generates `Allocator` implementation for given object.
macro_rules! impl_allocator_api {
    ($t:ty) => {
        unsafe impl crate::std::alloc::Allocator for $t {
            fn allocate(
                &self,
                layout: Layout,
            ) -> Result<NonNull<[u8]>, crate::std::alloc::AllocError> {
                self.allocate(layout)
                    .map_err(|_| crate::std::alloc::AllocError)
            }

            unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
                unsafe { self.deallocate(ptr, layout) }
            }
        }
    };
}

#[cfg(feature = "allocator-api")]
pub(crate) use impl_allocator_api;

pub trait Initialization {
    /// Constructs an uninitialized allocator that
    /// must be initialized via [`init`].
    ///
    /// [`init`]: #method.init
    fn new_uninitialized() -> Self;

    /// Returns `true` if allocator is initialized.
    fn is_initialized(&self) -> bool;

    /// # Safety
    /// - `size` follows alignment requirement for implemented allocator.
    /// - `size` not equal to zero.
    /// - `start` + `size` does not overflow [`usize::MAX`].
    unsafe fn init(&self, start: usize, size: usize);
}

pub trait Allocations {
    /// Returns number of allocations currently allocated.
    fn allocations(&self) -> usize;
}

pub trait Bytes {
    /// Returns number remaining bytes available in allocator.
    fn remaining_bytes(&self) -> usize;
    /// Returns number bytes allocated in allocator.
    fn allocated_bytes(&self) -> usize;
}
