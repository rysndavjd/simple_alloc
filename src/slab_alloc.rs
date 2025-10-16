use core::{
    alloc::{GlobalAlloc, Layout},
    mem::{MaybeUninit, align_of, size_of},
    ptr::{null_mut, NonNull},
};

use crate::{Locked, common::align_up};

#[derive(Debug)]
pub struct FreeList {
    pub next: Option<NonNull<FreeList>>,
    pub prev: Option<NonNull<FreeList>>
}

impl FreeList {
    const fn new() -> Self {
        Self { next: None , prev: None }
    }

    pub fn start_addr(&self) -> usize {
        self as *const Self as usize
    }
}

#[derive(Debug)]
pub struct FreeArea {
    pub head: Option<NonNull<FreeList>>,
    pub tail: Option<NonNull<FreeList>>,
    pub nr_free: usize,
}

impl FreeArea {
    const fn new() -> FreeArea {
        FreeArea {
            head: None,
            tail: None,
            nr_free: 0,
        }
    }

    fn push(&mut self, mut value: NonNull<FreeList>) {
        unsafe {
            // Link the old head to the new head next ptr
            value.as_mut().next = self.head;

            // If the list is not empty link the old head to the new head prev ptr
            if let Some(mut head) = self.head {
                head.as_mut().prev = Some(value)
            } else {
                // Else the list is empty so this is first node  
                // added so it becomes the tail of the list
                self.tail = Some(value)
            }
            value.as_mut().prev = None;
        }
        self.head = Some(value);
        self.nr_free += 1;
    }

    fn pop(&mut self) -> Option<NonNull<FreeList>> {
        if let Some(mut node) = self.head {
            unsafe {
                // Set head to the next node
                self.head = node.as_ref().next;

                // If head is not empty set its prev ptr to none
                if let Some(mut head) = self.head {
                    head.as_mut().prev = None;
                } else {
                    // Else set tail to none as head is empty.
                    self.tail = None;
                }
                node.as_mut().next = None;
                node.as_mut().prev = None;
            }
            self.nr_free -= 1;
            Some(node)
        } else {
            None
        }
    }
}
