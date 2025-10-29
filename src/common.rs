use core::{
    alloc::{Layout, LayoutError},
    fmt::{Debug, Formatter, Result as FmtResult},
    ptr::write_bytes,
};
use spin::{Mutex, MutexGuard};
use std::ptr::NonNull;

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

/// # Safety
/// This function is marked unsafe as it could read uninitialized memory causing
/// miri to get very mad and output a very long backtrace.
pub unsafe fn print_heap_dump(heap: *const u8, len: usize) {
    unsafe {
        for i in 0..len {
            if i % 16 == 0 {
                print!("\n{:08x}: ", i);
            }
            print!("{:02x} ", *heap.add(i));
        }
        println!();
    }
}

pub enum AllocatorError {
    Oom(Layout),
    Overflow,
    Alignment(Layout),
    Layout(LayoutError),
    Null,
}

impl Debug for AllocatorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            AllocatorError::Oom(layout) => write!(f, "Out of Memory: layout: {layout:?}"),
            AllocatorError::Overflow => write!(f, "Overflowed memory allocator."),
            AllocatorError::Alignment(layout) => {
                write!(f, "Unable to satisfy alignment requirement: {layout:?}")
            }
            AllocatorError::Layout(e) => write!(f, "Layout Error: {e:?}"),
            AllocatorError::Null => write!(f, "NULL pointer."),
        }
    }
}

/// # Safety
pub unsafe trait Allocator {
    /// # Safety
    unsafe fn try_allocate(&self, layout: Layout) -> Result<NonNull<u8>, AllocatorError>;

    /// # Safety
    unsafe fn try_deallocate(&self, ptr: NonNull<u8>, layout: Layout)
    -> Result<(), AllocatorError>;

    /// # Safety
    unsafe fn try_allocate_zeroed(&self, layout: Layout) -> Result<NonNull<u8>, AllocatorError> {
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
    ) -> Result<(), AllocatorError> {
        unsafe {
            write_bytes(ptr.as_ptr(), 0, layout.size());
            self.try_deallocate(ptr, layout)?;
        };
        return Ok(());
    }
}
