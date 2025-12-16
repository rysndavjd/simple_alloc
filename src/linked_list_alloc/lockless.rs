use core::{ptr::NonNull, sync::atomic::AtomicPtr};

#[derive(Debug)]
struct Node {
    size: usize,
    next: Option<AtomicPtr<Node>>,
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

pub struct LocklessLinkedList {
    head: Node,
}
