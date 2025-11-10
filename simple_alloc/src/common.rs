use core::{
    alloc::{GlobalAlloc, Layout, LayoutError},
    fmt::{Debug, Formatter, Result as FmtResult},
    marker::PhantomData,
    ptr::{NonNull, null_mut, write_bytes},
};
use spin::{Mutex, MutexGuard};

pub const ALLOCATOR_UNINITIALIZED: &str = "Allocator not initialized.";

pub struct Locked<A> {
    inner: Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, A> {
        self.inner.lock()
    }
}

pub fn align_up(addr: usize, align: usize) -> usize {
    let offset = (addr as *const u8).align_offset(align);
    addr + offset
}

pub enum BAllocatorError {
    Oom(Layout),
    Overflowed,
    Alignment(Layout),
    Layout(LayoutError),
    Null,
}

impl Debug for BAllocatorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            BAllocatorError::Oom(layout) => write!(f, "Out of Memory: {layout:?}"),
            BAllocatorError::Overflowed => write!(f, "Overflowed memory allocator internal values"),
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
    /// # Safety
    unsafe fn init(&self, start: usize, size: usize);

    /// # Safety
    unsafe fn try_allocate(&self, layout: Layout) -> Result<NonNull<u8>, BAllocatorError>;

    /// # Safety
    unsafe fn try_deallocate(
        &self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), BAllocatorError>;

    fn remaining(&self) -> usize;

    /// # Safety
    unsafe fn try_allocate_zeroed(&self, layout: Layout) -> Result<NonNull<u8>, BAllocatorError> {
        let size = layout.size();
        let ptr = unsafe { self.try_allocate(layout)? };

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

pub trait Strategy {}
pub struct LockedAlloc;
impl Strategy for LockedAlloc {}
pub struct LocklessAlloc;
impl Strategy for LocklessAlloc {}
pub struct ConstAlloc;
impl Strategy for ConstAlloc {}

pub struct Alloc<A: BAllocator, S: Strategy> {
    pub(crate) alloc: A,
    pub(crate) _strategy: PhantomData<S>,
}

unsafe impl<A: BAllocator, S: Strategy> BAllocator for Alloc<A, S> {
    unsafe fn init(&self, start: usize, size: usize) {
        unsafe {
            self.alloc.init(start, size);
        };
    }

    unsafe fn try_allocate(&self, layout: Layout) -> Result<NonNull<u8>, BAllocatorError> {
        unsafe {
            return self.alloc.try_allocate(layout);
        }
    }

    unsafe fn try_deallocate(
        &self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), BAllocatorError> {
        unsafe {
            return self.alloc.try_deallocate(ptr, layout);
        }
    }

    fn remaining(&self) -> usize {
        return self.alloc.remaining();
    }
}

unsafe impl<A: BAllocator, S: Strategy> GlobalAlloc for Alloc<A, S> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            match self.alloc.try_allocate(layout) {
                Ok(mut ptr) => return ptr.as_mut(),
                Err(_) => return null_mut(),
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        assert!(!ptr.is_null(), "Given pointer to deallocate is NULL.");
        unsafe {
            self.alloc
                .try_deallocate(NonNull::new_unchecked(ptr), layout)
                .unwrap()
        }
    }
}
