use conquer_once::spin::OnceCell;

use crate::{
    common::{ALLOCATOR_UNINITIALIZED, BAllocator, BAllocatorError},
    std::{
        alloc::Layout,
        ptr::{NonNull, null_mut},
        sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
    },
};

#[derive(Debug)]
struct Node {
    size: usize,
    next: AtomicPtr<Node>,
}

impl Node {
    const fn new(size: usize) -> Self {
        Self {
            size,
            next: AtomicPtr::new(null_mut()),
        }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct LocklessLinkedList {
    head: Node,
    allocations: AtomicUsize,
}

impl LocklessLinkedList {
    const fn new() -> Self {
        Self {
            head: Node::new(0),
            allocations: AtomicUsize::new(0),
        }
    }

    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(align_of::<Node>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(size_of::<Node>());
        (size, layout.align())
    }
}

unsafe impl BAllocator for OnceCell<LocklessLinkedList> {
    fn try_allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, BAllocatorError> {
        assert!(self.is_initialized(), "{ALLOCATOR_UNINITIALIZED}");

        let (size, align) = LocklessLinkedList::size_align(layout);
        let mut allocator = unsafe { self.get_unchecked() };

        todo!()
    }

    fn try_deallocate(&self, ptr: NonNull<[u8]>, layout: Layout) -> Result<(), BAllocatorError> {
        todo!()
    }
}
