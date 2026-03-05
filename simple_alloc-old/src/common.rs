use crate::std::{
    alloc::{AllocError, Allocator, GlobalAlloc, Layout, LayoutError},
    fmt::{Debug, Formatter, Result as FmtResult},
    ptr::NonNull,
};

pub const HEAP_START_NULL: &str = "Given heap start pointer is NULL";
pub const HEAP_SIZE_ZERO: &str = "Heap cannot be 0 in size";
pub const HEAP_END_OVERFLOWED: &str = "Heap end address overflowed";
pub const HEAP_NOT_POWER_TWO: &str = "Heap not a power of 2";
pub const ALLOCATOR_UNINITIALIZED: &str = "Allocator not initialized";
pub const ALLOCATOR_ALREADY_INITIALIZED: &str = "Allocator was already initialized";
pub const OOM: &str = "Out of memory";
pub const NON_ZERO_LAYOUT: &str = "Layout must have non-zero size";

// assert!(layout.size() != 0, "{NON_ZERO_LAYOUT}");

pub fn align_up(addr: usize, align: usize) -> usize {
    let offset = (addr as *const u8).align_offset(align);
    addr + offset
}

pub enum BAllocatorError {
    Oom,
    Overflowed,
    Underflowed,
    Alignment(Layout),
    Layout(LayoutError),
    Null,
}

impl Debug for BAllocatorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            BAllocatorError::Oom => write!(f, "Out of memory"),
            BAllocatorError::Overflowed => write!(f, "Overflowed memory allocator internal values"),
            BAllocatorError::Underflowed => {
                write!(f, "Underflowed memory allocator internal values")
            }
            BAllocatorError::Alignment(layout) => {
                write!(f, "Unable to satisfy alignment requirement: {layout:?}")
            }
            BAllocatorError::Layout(e) => write!(f, "Layout Error: {e:?}"),
            BAllocatorError::Null => write!(f, "NULL pointer"),
        }
    }
}

/// # Safety
pub unsafe trait BAllocator {
    fn try_allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, BAllocatorError>;

    fn allocate_zerod(&self, layout: Layout) -> Result<NonNull<[u8]>, BAllocatorError> {
        let ptr = self.try_allocate(layout)?;

        // SAFTY: `try_allocate` returns a valid memory block
        unsafe {
            (ptr.as_ptr() as *mut u8).write_bytes(0, ptr.len());
        }

        return Ok(ptr);
    }

    fn deallocate(&self, ptr: NonNull<u8>, layout: Layout);
}

pub trait AllocInit {
    fn is_initialized(&self) -> bool;

    /// # Safety
    unsafe fn init(&self, start: usize, size: usize);
}

impl<A: BAllocator + AllocInit> AllocInit for Alloc<A> {
    fn is_initialized(&self) -> bool {
        self.alloc.is_initialized()
    }

    unsafe fn init(&self, start: usize, size: usize) {
        unsafe { self.alloc.init(start, size) };
    }
}

pub trait Allocations {
    fn allocations(&self) -> usize;
}

impl<A: BAllocator + Allocations> Allocations for Alloc<A> {
    fn allocations(&self) -> usize {
        return self.alloc.allocations();
    }
}

pub struct Alloc<A: BAllocator> {
    pub(crate) alloc: A,
}

// unsafe impl<A: BAllocator> GlobalAlloc for Alloc<A> {
//     unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
//         match self.alloc.try_allocate(layout) {
//             Ok(ptr) => return ptr.as_ptr() as *mut u8,
//             Err(_e) => {
//                 #[cfg(debug_assertions)]
//                 error!("GlobalAlloc, Allocation error: {:?}", _e);
//             }
//         }
//         handle_alloc_error(layout);
//     }

//     unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
//         assert!(!ptr.is_null(), "Given pointer to deallocate is NULL");
//         unsafe {
//             {
//                 if let Err(_e) = self.alloc.deallocate(
//                     NonNull::new_unchecked(slice_from_raw_parts_mut(ptr, layout.size())),
//                     layout,
//                 ) {
//                     #[cfg(debug_assertions)]
//                     error!("GlobalAlloc, Deallocation error: {:?}", _e);
//                     handle_alloc_error(layout)
//                 }
//             }
//         }
//     }
// }

// unsafe impl<A: BAllocator> Allocator for Alloc<A> {
//     fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
//         match self.alloc.try_allocate(layout) {
//             Ok(ptr) => {},
//             Err(e) =>
//         }
//     }

//     unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {}
// }
