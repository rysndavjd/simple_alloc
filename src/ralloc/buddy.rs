use spin::Mutex;

use crate::{
    ralloc::{PAGE_SHIFT, PAGE_SIZE},
    std::{
        alloc::Layout,
        ptr::{NonNull, slice_from_raw_parts_mut},
    },
};

const fn max_order(heap_size: usize) -> usize {
    (heap_size / PAGE_SIZE).ilog2() as usize
}

enum BuddyError {
    OrderZeroCannotBeSplit,
    SourceAreaEmpty,
    InvalidLayout,
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
    /// Aligns given layout to [`PAGE_SIZE`] and returns how
    /// many pages this layout takes up.
    fn size_align_pages(layout: Layout) -> Result<usize, BuddyError> {
        let aligned_layout = layout
            .align_to(PAGE_SIZE)
            .map_err(|_| BuddyError::InvalidLayout)?
            .pad_to_align();

        return Ok(aligned_layout.size().div_ceil(PAGE_SIZE));
    }

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

    fn split_down(&mut self, source_order: usize) -> Result<(), BuddyError> {
        if source_order == 0 {
            return Err(BuddyError::OrderZeroCannotBeSplit);
        }

        let source_chunk = {
            let mut source_area = self.areas_list[source_order].lock();

            if source_area.nr_free == 0 {
                return Err(BuddyError::SourceAreaEmpty);
            }

            source_area
                .pop()
                .expect("Source area should contain atleast 1 chunk")
        };

        let mut target_area = self.areas_list[source_order - 1].lock();

        unsafe {
            let buddy = NonNull::new_unchecked(
                source_chunk.as_ref().buddy_addr(source_order - 1) as *mut Chunk
            );
            buddy.write(Chunk::new());

            target_area.push(source_chunk);
            target_area.push(buddy);
        }

        return Ok(());
    }

    /// Allocates `nr_pages` pages and returns a fat pointer.
    /// The pointer is aligned to [`PAGE_SIZE`], and its [`len`] represents
    /// the number of pages covered by the allocation.
    fn allocate_page(&self, nr_pages: usize) -> Result<NonNull<[u8]>, BuddyError> {
        let order = nr_pages.ilog2() as usize;

        {
            let mut first_order = self.areas_list[order].lock();

            if first_order.nr_free != 0 {
                let chunk = first_order
                    .pop()
                    .expect("Source area should contain atleast 1 chunk");

                unsafe {
                    return Ok(NonNull::new_unchecked(slice_from_raw_parts_mut(
                        chunk.as_ptr() as *mut u8,
                        1 << order,
                    )));
                };
            }
        }

        todo!()
    }

    fn deallocate(&self, ptr: NonNull<[u8]>) {
        todo!()
    }
}
