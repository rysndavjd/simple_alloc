use core::{
    alloc::{GlobalAlloc, Layout},
    mem::{MaybeUninit, align_of, size_of},
    ptr::{NonNull, null_mut},
};

use crate::common::{Locked, align_up};

#[derive(Debug)]
pub struct FreeList {
    pub next: Option<NonNull<FreeList>>,
    pub prev: Option<NonNull<FreeList>>,
}

impl FreeList {
    const fn new() -> FreeList {
        FreeList {
            next: None,
            prev: None,
        }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn push_head(&mut self, mut value: NonNull<FreeList>) {
        unsafe {
            // Link the old head to the new head next ptr
            value.as_mut().next = self.next;

            // If the list is not empty link the old head to the new head prev ptr
            if let Some(mut head) = self.next {
                head.as_mut().prev = Some(value)
            } else {
                // Else the list is empty so this is first node
                // added so it becomes the tail of the list
                self.prev = Some(value)
            }
            value.as_mut().prev = None;
        }
        self.next = Some(value);
    }

    fn pop_head(&mut self) -> Option<NonNull<FreeList>> {
        if let Some(mut node) = self.next {
            unsafe {
                // Set head to the next node
                self.next = node.as_ref().next;

                // If head is not empty set its prev ptr to none
                if let Some(mut head) = self.next {
                    head.as_mut().prev = None;
                } else {
                    // Else set tail to none as head is empty.
                    self.prev = None;
                }
                node.as_mut().next = None;
                node.as_mut().prev = None;
            }
            Some(node)
        } else {
            None
        }
    }
}

struct Slab {
    next: Option<NonNull<Slab>>,
    prev: Option<NonNull<Slab>>,
    object_size: usize,
    object_list: Option<NonNull<FreeList>>,
    free_object_count: usize,
    max_object_count: usize,
    slab_size: usize,
    slab_state: usize,
}

impl Slab {
    const SLAB_EMPTY: usize = 0;
    const SLAB_PARTIAL: usize = 1;
    const SLAB_FULL: usize = 2;

    const fn new() -> Slab {
        Slab {
            next: None,
            prev: None,
            object_size: 0,
            object_list: None,
            free_object_count: 0,
            max_object_count: 0,
            slab_size: 0,
            slab_state: 0,
        }
    }

    fn start_addr(&self) -> usize {
        return self as *const Self as usize;
    }

    fn end_addr(&self) -> usize {
        return self.start_addr() + self.slab_size;
    }

    fn push_head(&mut self, mut value: NonNull<Slab>) {
        unsafe {
            value.as_mut().next = self.next;

            if let Some(mut head) = self.next {
                head.as_mut().prev = Some(value)
            } else {
                self.prev = Some(value)
            }
            value.as_mut().prev = None;
        }
        self.next = Some(value);
    }

    fn pop_head(&mut self) -> Option<NonNull<Slab>> {
        if let Some(mut node) = self.next {
            unsafe {
                self.next = node.as_ref().next;

                if let Some(mut head) = self.next {
                    head.as_mut().prev = None;
                } else {
                    self.prev = None;
                }
                node.as_mut().next = None;
                node.as_mut().prev = None;
            }
            Some(node)
        } else {
            None
        }
    }
}

const PAGE_SIZE: usize = 4096;

pub struct SlabAlloc {
    base: *mut u8,
    size: usize,
    slab_lists: Option<NonNull<Slab>>,
}
