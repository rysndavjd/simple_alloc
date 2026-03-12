use spin::Mutex;

use crate::{
    ralloc::{PAGE_SHIFT, PAGE_SIZE},
    std::{
        ptr::NonNull,
        sync::atomic::{AtomicPtr, Ordering},
    },
};

const fn max_order(heap_size: usize) -> usize {
    (heap_size / PAGE_SIZE).ilog2() as usize
}

enum BuddyErrors {
    AreaOrderEmpty,
}

struct Chunk {
    next: Option<NonNull<Chunk>>,
}

impl Chunk {
    const fn new() -> Self {
        Self { next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self, order: usize) -> usize {
        self.start_addr() + (1 << (order + PAGE_SHIFT))
    }

    fn buddy_addr(&self, order: usize) -> usize {
        self.start_addr() ^ (PAGE_SIZE << order)
    }
}

struct Chunks {
    head: Option<NonNull<Chunk>>,
    nr_free: usize,
}

impl Chunks {
    const fn new() -> Chunks {
        Chunks {
            head: None,
            nr_free: 0,
        }
    }

    fn push(&mut self, mut chunk: NonNull<Chunk>) {
        unsafe {
            chunk.as_mut().next = self.head;
        }
        self.head = Some(chunk);
        self.nr_free += 1;
    }

    fn pop(&mut self) -> Option<NonNull<Chunk>> {
        if let Some(mut old_head) = self.head {
            unsafe {
                self.head = old_head.as_ref().next;
                self.nr_free -= 1;

                old_head.as_mut().next = None;
            }

            return Some(old_head);
        }
        return None;
    }
}

pub struct Buddy<const NR_ORDER: usize> {
    start: usize,
    size: usize,
    areas_list: [Mutex<Chunks>; NR_ORDER],
}

impl<const NR_ORDER: usize> Buddy<NR_ORDER> {
    fn add_area(&mut self, addr: NonNull<u8>, order: usize) {
        assert!(addr.as_ptr() as usize & (PAGE_SIZE - 1) == 0);

        let mut area = self.areas_list[order].lock();

        let mut new_chunk = Chunk::new();
        new_chunk.next = area.head;

        let chunk_ptr = addr.as_ptr() as *mut Chunk;

        unsafe {
            chunk_ptr.write_volatile(new_chunk);
            area.head = NonNull::new(chunk_ptr);
            area.nr_free += 1;
        }
    }

    fn split(&mut self, source_order: usize, target_order: usize) -> Result<(), BuddyErrors> {
        debug_assert!(source_order > target_order);

        let mut source_area = self.areas_list[source_order].lock();

        if source_area.nr_free == 0 {
            return Err(BuddyErrors::AreaOrderEmpty);
        }

        let mut target_area = self.areas_list[target_order].lock();

        let source_chunk = source_area
            .pop()
            .expect("Source area should contain atleast 1 chunk");

        unsafe {
            let buddy_addr = source_chunk.as_ref().buddy_addr(target_order);

            todo!()
        }
    }
}
