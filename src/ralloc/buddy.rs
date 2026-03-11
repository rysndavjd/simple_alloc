use spin::Mutex;

use crate::{
    ralloc::PAGE_SIZE,
    std::{ptr::NonNull, sync::atomic::AtomicPtr},
};

const fn max_order(heap_size: usize) -> usize {
    (heap_size / PAGE_SIZE).ilog2() as usize
}

/// A chunk is a 4096 byte memory region without any metadata attached to it.
struct Chunk {
    next: Option<NonNull<Chunk>>,
    prev: Option<NonNull<Chunk>>,
}

struct Chunks {
    head: Option<NonNull<Chunk>>,
    tail: Option<NonNull<Chunk>>,
}

pub struct Buddy<const MAX_ORDER: usize> {
    start: usize,
    size: usize,
    order_zero: AtomicPtr<Chunk>,
    areas_list: [Mutex<Chunks>; MAX_ORDER],
}
