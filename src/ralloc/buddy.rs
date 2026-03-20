use crate::{
    ralloc::{PAGE_SHIFT, PAGE_SIZE},
    std::{
        alloc::Layout,
        ptr::{NonNull, slice_from_raw_parts_mut},
    },
};

enum BuddyError {
    OrderZeroCannotBeSplit,
    OrderGreaterThanMax,
    SourceAreaEmpty,
    SourceAreaMissingABuddy,
    InvalidLayout,
    Oom,
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

// Eventually use a red black tree for sorting
// available pages in list of chunks
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
            let chunk_addr = chunk.as_ref().start_addr();

            let mut prev = None;
            let mut curr = self.head;

            while let Some(c) = curr {
                if c.as_ref().start_addr() > chunk_addr {
                    break;
                }
                prev = curr;
                curr = c.as_ref().next;
            }

            chunk.as_mut().next = curr;

            match prev {
                None => self.head = Some(chunk),
                Some(mut p) => p.as_mut().next = Some(chunk),
            }

            self.nr_free += 1;
        }
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
    areas_list: [Chunks; NR_ORDER],
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

        let mut new_chunk = Chunk::new();
        new_chunk.next = self.areas_list[order].head;

        let chunk_ptr = addr.as_ptr() as *mut Chunk;

        unsafe {
            chunk_ptr.write_volatile(new_chunk);
            self.areas_list[order].head = NonNull::new(chunk_ptr);
            self.areas_list[order].nr_free += 1;
        }
    }

    /// Takes `source_order` and splits that order into `source_order - 1`
    fn split_down(&mut self, source_order: usize) -> Result<(), BuddyError> {
        if source_order == 0 {
            return Err(BuddyError::OrderZeroCannotBeSplit);
        }

        let source_chunk = {
            let source_area = &mut self.areas_list[source_order];

            if source_area.nr_free == 0 {
                return Err(BuddyError::SourceAreaEmpty);
            }

            source_area
                .pop()
                .expect("Source area should contain atleast 1 chunk")
        };

        let target_area = &mut self.areas_list[source_order - 1];

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

    /// Takes `source_order` and combines two buddies if possible and adds it to `source_order + 1`
    fn combine_up(&mut self, source_order: usize) -> Result<(), BuddyError> {
        if source_order > NR_ORDER - 1 {
            return Err(BuddyError::OrderGreaterThanMax);
        }

        if self.areas_list[source_order].nr_free < 2 {
            return Err(BuddyError::SourceAreaMissingABuddy);
        }

        let head = self.areas_list[source_order]
            .head
            .expect("Source area should contain atleast 2 chunks, head");
        let next = unsafe { head.as_ref().next }
            .expect("Source area should contain atleast 2 chunks, next");

        unsafe {
            if head.as_ref().buddy_addr(source_order) == next.as_ref().start_addr() {
                let lower = self.areas_list[source_order].pop().unwrap();
                self.areas_list[source_order].pop();
                self.areas_list[source_order + 1].push(lower);
            }
        }

        Ok(())
    }

    /// Allocates `nr_pages` pages and returns a fat pointer.
    /// The pointer is aligned to [`PAGE_SIZE`], and its [`len`] represents
    /// the number of bytes covered by the allocation.
    fn allocate_page(&mut self, nr_pages: usize) -> Result<NonNull<[u8]>, BuddyError> {
        let alloc_order = nr_pages.next_power_of_two().ilog2() as usize;

        if alloc_order > (NR_ORDER - 1) {
            return Err(BuddyError::Oom);
        }

        let lowest_avail_order = (alloc_order..NR_ORDER)
            .find(|&order| self.areas_list[order].nr_free > 0)
            .ok_or(BuddyError::Oom)?;

        if lowest_avail_order == alloc_order {
            let chunk = self.areas_list[alloc_order]
                .pop()
                .expect("Source area should contain atleast 1 chunk");

            unsafe {
                return Ok(NonNull::new_unchecked(slice_from_raw_parts_mut(
                    chunk.as_ptr() as *mut u8,
                    PAGE_SIZE << alloc_order,
                )));
            };
        }

        for order in ((alloc_order + 1)..=lowest_avail_order).rev() {
            self.split_down(order)?;
        }

        let chunk = self.areas_list[alloc_order]
            .pop()
            .expect("Source area should contain atleast 1 chunk");

        unsafe {
            return Ok(NonNull::new_unchecked(slice_from_raw_parts_mut(
                chunk.as_ptr() as *mut u8,
                PAGE_SIZE << alloc_order,
            )));
        };
    }

    fn deallocate_page(&self, ptr: NonNull<u8>, nr_pages: usize) {
        todo!()
    }
}
