use core::{
    alloc::{GlobalAlloc, Layout, LayoutError},
    fmt::{Debug, Formatter, Result as FmtResult},
    ptr::{NonNull, null_mut, write_bytes},
};

#[cfg(debug_assertions)]
use log::error;

pub const HEAP_START_NULL: &str = "Given heap start pointer is NULL";
pub const HEAP_SIZE_ZERO: &str = "Heap cannot be 0 in size";
pub const HEAP_END_OVERFLOWED: &str = "Heap end address overflowed";
pub const ALLOCATOR_UNINITIALIZED: &str = "Allocator not initialized";
pub const OOM: &str = "Out of memory";

pub fn align_up(addr: usize, align: usize) -> usize {
    let offset = (addr as *const u8).align_offset(align);
    addr + offset
}

pub enum BAllocatorError {
    Oom(Option<Layout>),
    Overflowed,
    Underflowed,
    Alignment(Layout),
    Layout(LayoutError),
    Null,
}

impl Debug for BAllocatorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            BAllocatorError::Oom(layout) => write!(f, "Out of memory: {layout:?}"),
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
    fn try_allocate(&self, layout: Layout) -> Result<NonNull<u8>, BAllocatorError>;

    fn try_deallocate(&self, ptr: NonNull<u8>, layout: Layout) -> Result<(), BAllocatorError>;

    fn try_allocate_zeroed(&self, layout: Layout) -> Result<NonNull<u8>, BAllocatorError> {
        let size = layout.size();
        let ptr = self.try_allocate(layout)?;

        unsafe { write_bytes(ptr.as_ptr(), 0, size) };

        return Ok(ptr);
    }

    /// # Safety
    unsafe fn try_deallocate_zeroed(
        &self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), BAllocatorError> {
        unsafe {
            write_bytes(ptr.as_ptr(), 0, layout.size());
            self.try_deallocate(ptr, layout)?;
        };
        return Ok(());
    }
}

pub trait AllocInit {
    fn is_initialized(&self) -> bool;

    /// # Safety
    ///
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

pub trait AllocState {
    fn remaining(&self) -> usize;
    fn allocations(&self) -> usize;
}

impl<A: BAllocator + AllocState> AllocState for Alloc<A> {
    fn remaining(&self) -> usize {
        return self.alloc.remaining();
    }

    fn allocations(&self) -> usize {
        return self.alloc.allocations();
    }
}

pub struct Alloc<A: BAllocator> {
    pub(crate) alloc: A,
}

unsafe impl<A: BAllocator> BAllocator for Alloc<A> {
    fn try_allocate(&self, layout: Layout) -> Result<NonNull<u8>, BAllocatorError> {
        return self.alloc.try_allocate(layout);
    }

    fn try_deallocate(&self, ptr: NonNull<u8>, layout: Layout) -> Result<(), BAllocatorError> {
        return self.alloc.try_deallocate(ptr, layout);
    }
}

unsafe impl<A: BAllocator> GlobalAlloc for Alloc<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            match self.alloc.try_allocate(layout) {
                Ok(mut ptr) => return ptr.as_mut(),
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    error!("GlobalAlloc, Allocation error: {:?}", _e);
                    return null_mut();
                }
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        assert!(!ptr.is_null(), "Given pointer to deallocate is NULL.");
        unsafe {
            #[cfg(not(debug_assertions))]
            {
                let _ = self
                    .alloc
                    .try_deallocate(NonNull::new_unchecked(ptr), layout);
            }

            #[cfg(debug_assertions)]
            {
                if let Err(e) = self
                    .alloc
                    .try_deallocate(NonNull::new_unchecked(ptr), layout)
                {
                    error!("GlobalAlloc, Deallocation error: {:?}", e)
                }
            }
        }
    }
}
