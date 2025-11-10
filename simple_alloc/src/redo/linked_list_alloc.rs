// Portions copyright (c) Philipp Oppermann (https://os.phil-opp.com/)
// Licensed under MIT OR Apache-2.0

use core::{
    alloc::{GlobalAlloc, Layout},
    mem::{align_of, size_of},
    ptr::null_mut,
};

use crate::common::{Locked, align_up};

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

pub struct LinkedListAlloc {
    head: Node,
}

impl Default for LinkedListAlloc {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkedListAlloc {
    /// Creates a new empty [`LinkedListAlloc`]
    pub const fn new() -> Self {
        Self { head: Node::new(0) }
    }

    /// Initializes the linked list allocator with the given heap bounds.
    ///
    /// # Safety
    /// - Must be called only once.
    /// - The heap memory region must have at least 8 bytes available per allocation
    ///   to store linked list metadata.
    /// - The heap must be 8 byte aligned.    
    /// - `heap_start` must be valid memory address (NON-NULL).
    /// - `heap_size` must be greater than 0.
    /// - `heap_start + heap_size` must not overflow.
    /// - The caller must ensure exclusive access to provided memory region for the lifetime of the allocator.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        debug_assert!(heap_size > 0, "Linked list heap cannot be zero in size.");
        debug_assert_eq!(
            align_up(heap_start, align_of::<Node>()),
            heap_start,
            "Given heap start is not 8 byte aligned."
        );
        unsafe {
            self.add_free_region(heap_start, heap_size);
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
            node_ptr.write(new_node);
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

unsafe impl GlobalAlloc for Locked<LinkedListAlloc> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = LinkedListAlloc::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = match alloc_start.checked_add(size) {
                Some(t) => t,
                None => return null_mut(),
            };
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                unsafe {
                    allocator.add_free_region(alloc_end, excess_size);
                }
            }
            return alloc_start as *mut u8;
        } else {
            return null_mut();
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (size, _) = LinkedListAlloc::size_align(layout);

        unsafe {
            self.lock().add_free_region(ptr as usize, size);
            self.lock().combine_free_regions();
        }
    }
}
