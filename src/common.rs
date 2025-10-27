use spin::{Mutex, MutexGuard};
use core::{ptr::write_bytes, fmt::{Debug, Display, Result as FmtResult, Formatter}, alloc::{Layout, LayoutError}};

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
    Oom (Layout),
    Alignment(Layout),
    Layout(LayoutError),
}

impl Debug for AllocatorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            AllocatorError::Oom(layout) => write!(f, "Out of Memory: layout: {layout:?}"),
            AllocatorError::Alignment(layout) => write!(f, "Unable to satisfy alignment requirement: {layout:?}"),
            AllocatorError::Layout(e) => write!(f, "Layout Error: {e:?}"),
        }
    }
}

/// # Safety
pub unsafe trait Allocator {
    /// # Safety
    unsafe fn try_allocate(&self, layout: Layout) -> Result<*mut u8, AllocatorError>;

    /// # Safety
    unsafe fn try_deallocate(&self, ptr: *mut u8, layout: Layout) -> Result<(), AllocatorError>;

    /// # Safety
    unsafe fn try_allocate_zeroed(&self, layout: Layout) -> Result<*mut u8, AllocatorError> {
        let size = layout.size();
        let ptr = unsafe { self.try_allocate(layout)? };
        if !ptr.is_null() {
            unsafe { write_bytes(ptr, 0, size) };
        } else {
            return Err(AllocatorError::Oom(layout));
        };
        return Ok(ptr);
    }

    /// # Safety
    unsafe fn try_deallocate_zeroed(&self, ptr: *mut u8, layout: Layout) -> Result<(), AllocatorError> {
        unsafe {
            write_bytes(ptr, 0, layout.size());
            self.try_deallocate(ptr, layout)?;
        };
        return Ok(());
    }
}