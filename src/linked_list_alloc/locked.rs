// Portions copyright (c) Philipp Oppermann (https://os.phil-opp.com/)
// Licensed under MIT OR Apache-2.0

use core::{
    alloc::Layout,
    mem::{align_of, size_of},
    ptr::NonNull,
};

#[cfg(debug_assertions)]
use log::{debug, trace};
use spin::Mutex;

use crate::common::{
    Alloc, AllocInit, BAllocator, BAllocatorError, HEAP_END_OVERFLOWED, HEAP_SIZE_ZERO,
    HEAP_START_NULL, align_up,
};

#[derive(Debug)]
struct Node {
    size: usize,
    next: Option<&'static mut Node>,
}

impl Node {
    const fn new(size: usize) -> Self {
        Self { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct LockedLinkedList {
    head: Node,
}

impl Default for LockedLinkedList {
    fn default() -> Self {
        Self::new()
    }
}

impl LockedLinkedList {
    const fn new() -> Self {
        Self { head: Node::new(0) }
    }

    unsafe fn init(&mut self, start: usize, size: usize) {
        debug_assert!(start != 0, "{}", HEAP_START_NULL);
        debug_assert!(size > 0, "{}", HEAP_SIZE_ZERO);
        debug_assert!(start + size < usize::MAX, "{}", HEAP_END_OVERFLOWED);
        debug_assert_eq!(
            align_up(start, align_of::<Node>()),
            start,
            "Given start is not 8 byte aligned"
        );
        unsafe {
            self.add_free_region(start, size);
        }
    }

    unsafe fn combine_free_regions(&mut self) {
        let mut current = &mut self.head;

        while let Some(ref mut node) = current.next {
            let node_start = node.start_addr();
            if let Some(ref mut next) = node.next
                && node_start + node.size == next.start_addr()
            {
                node.size += next.size;
                node.next = next.next.take();
            }
            current = node;
        }
    }

    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        assert_eq!(align_up(addr, align_of::<Node>()), addr);
        assert!(size >= size_of::<Node>());

        let mut new_node = Node::new(size);
        new_node.next = self.head.next.take();
        let node_ptr = addr as *mut Node;

        unsafe {
            #[cfg(debug_assertions)]
            trace!(
                "Added free region: {:?}, at Addr: {:#X}",
                new_node, node_ptr as usize
            );
            node_ptr.write_volatile(new_node);
            self.head.next = Some(&mut *node_ptr)
        }
    }

    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut Node, usize)> {
        let mut current = &mut self.head;

        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(region, size, align) {
                let next = region.next.take();
                let ret = Some((current.next.take()?, alloc_start));
                current.next = next;
                return ret;
            } else {
                current = current.next.as_mut()?
            }
        }

        return None;
    }

    fn alloc_from_region(region: &Node, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < size_of::<Node>() {
            return Err(());
        }

        Ok(alloc_start)
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

unsafe impl BAllocator for Mutex<LockedLinkedList> {
    unsafe fn try_allocate(&self, layout: Layout) -> Result<NonNull<u8>, BAllocatorError> {
        let (size, align) = LockedLinkedList::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = match alloc_start.checked_add(size) {
                Some(t) => t,
                None => return Err(BAllocatorError::Oom(Some(layout))),
            };
            match region.end_addr().checked_sub(alloc_end) {
                Some(excess_size) => unsafe {
                    allocator.add_free_region(alloc_end, excess_size);
                },
                None => return Err(BAllocatorError::Underflowed),
            }

            return Ok(unsafe { NonNull::new_unchecked(alloc_start as *mut u8) });
        } else {
            return Err(BAllocatorError::Oom(Some(layout)));
        }
    }

    unsafe fn try_deallocate(
        &self,
        ptr: core::ptr::NonNull<u8>,
        layout: Layout,
    ) -> Result<(), BAllocatorError> {
        let (size, _) = LockedLinkedList::size_align(layout);

        unsafe {
            self.lock().add_free_region(ptr.as_ptr() as usize, size);
            self.lock().combine_free_regions();
        }
        return Ok(());
    }
}

unsafe impl Sync for Alloc<Mutex<LockedLinkedList>> {}
unsafe impl Send for Alloc<Mutex<LockedLinkedList>> {}

impl Alloc<Mutex<LockedLinkedList>> {
    pub const fn new() -> Self {
        Alloc {
            alloc: Mutex::new(LockedLinkedList::new()),
        }
    }
}

impl Default for Alloc<Mutex<LockedLinkedList>> {
    fn default() -> Self {
        Self::new()
    }
}

impl AllocInit for Mutex<LockedLinkedList> {
    unsafe fn init(&self, start: usize, size: usize) {
        unsafe {
            #[cfg(debug_assertions)]
            debug!("Initialized locked linked list alloc; start: {start:#X}, size: {size}");
            self.lock().init(start, size);
        }
    }
}
