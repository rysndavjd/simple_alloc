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

pub enum AllocatorError {
    Oom,
    InternalOverflow,
    Alignment(Layout),
}

impl Debug for AllocatorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            AllocatorError::Oom => write!(f, "Out of memory"),
            AllocatorError::InternalOverflow => write!(f, "Overflowed memory allocator internals"),
            AllocatorError::Alignment(layout) => {
                write!(f, "Unable to satisfy alignment requirement: {layout:?}")
            }
        }
    }
}

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
    fn new_uninitialized() -> Self;

    fn is_initialized(&self) -> bool;

    /// # Safety
    unsafe fn init(&self, start: usize, size: usize);
}

pub trait Allocations {
    fn allocations(&self) -> usize;
}

pub trait Bytes {
    fn remaining_bytes(&self) -> usize;
    fn allocated_bytes(&self) -> usize;
}
