use core::{
    alloc::{GlobalAlloc, Layout},
    fmt::{Debug, Formatter, Result as FmtResult},
    mem::{MaybeUninit, align_of, size_of},
    ptr::{NonNull, null_mut},
};

use crate::{Locked, common::align_up};

#[derive(Debug)]
pub struct FreeList {
    pub next: Option<NonNull<FreeList>>,
}

impl FreeList {
    const fn new() -> Self {
        Self { next: None }
    }

    pub fn start_addr(&self) -> usize {
        self as *const Self as usize
    }
}

#[derive(Debug)]
pub struct FreeArea {
    pub head: Option<NonNull<FreeList>>,
    pub nr_free: usize,
}

impl FreeArea {
    const fn new() -> FreeArea {
        FreeArea {
            head: None,
            nr_free: 0,
        }
    }

    fn push(&mut self, mut value: NonNull<FreeList>) {
        unsafe {
            value.as_mut().next = self.head;
        }
        self.head = Some(value);
        self.nr_free += 1;
    }

    fn pop(&mut self) -> Option<NonNull<FreeList>> {
        if let Some(mut node) = self.head {
            unsafe {
                self.head = node.as_ref().next;
                node.as_mut().next = None;
            }
            self.nr_free -= 1;
            Some(node)
        } else {
            None
        }
    }
}

pub const PAGE_SIZE: usize = 16;
pub const MIN_ORDER: usize = 1;
pub const MAX_ORDER: usize = 16;
pub const NR_MAX_ORDER: usize = MAX_ORDER + 1;

pub struct BuddyAlloc {
    base: *mut u8,
    size: usize,
    pub list_areas: [FreeArea; NR_MAX_ORDER],
}

impl Debug for BuddyAlloc {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "BuddyAlloc {{")?;
        writeln!(f, "    base: {:?}", self.base)?;
        writeln!(f, "    size: {}", self.size)?;
        writeln!(f, "    list_areas: [")?;
        for (i, v) in self.list_areas.iter().enumerate() {
            writeln!(f, "    {}: {:?}", i, v)?;
        }
        writeln!(f, "]}}")
    }
}

impl Default for BuddyAlloc {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: unsafe impl Send for BuddyAlloc {}

impl BuddyAlloc {
    pub const fn new() -> BuddyAlloc {
        BuddyAlloc {
            base: null_mut(),
            size: 0,
            list_areas: [const { FreeArea::new() }; NR_MAX_ORDER],
        }
    }

    pub fn init(&mut self, start: usize, size: usize) {
        assert!(start != 0, "Given start for heap is NULL.");
        assert!(
            size.is_power_of_two(),
            "Buddy Allocator heap not a power of two."
        );
        assert_eq!(
            align_up(start, align_of::<FreeList>()),
            start,
            "Given start is not 8 byte aligned."
        );

        self.base = start as *mut u8;
        self.size = size;

        unsafe {
            self.add_free_area(start, size.div_ceil(PAGE_SIZE).ilog2() as usize);
        }
    }

    /// # Safety
    /// TESTING
    pub unsafe fn add_free_area(&mut self, addr: usize, order: usize) {
        assert!(addr != 0, "Given free area has a NULL address pointer.");
        assert_eq!(align_up(addr, align_of::<FreeList>()), addr);

        let mut new_item = FreeList::new();
        new_item.next = self.list_areas[order].head;

        let node_ptr = addr as *mut FreeList;

        unsafe {
            node_ptr.write_volatile(new_item);
            self.list_areas[order].head = NonNull::new(node_ptr);
            self.list_areas[order].nr_free += 1;
        }
    }

    /*
     * I am lazy to make proper errors as the error would either cause a panic
     * or return error if there is no more space left.
     */
    #[allow(clippy::result_unit_err)]
    pub fn split_area_to(&mut self, target_order: usize) -> Result<(), ()> {
        let source_order = (target_order..NR_MAX_ORDER)
            .find(|&order| self.list_areas[order].nr_free > 0)
            .ok_or(())?;

        for current_order in (target_order..=source_order).rev() {
            if self.list_areas[current_order].nr_free > 0 {
                if current_order == target_order {
                    return Ok(());
                }
                let area = self.list_areas[current_order].pop().ok_or(())?;

                let buddy_order = current_order
                    .checked_sub(1) // This should normally never underflow but checking just in case.
                    .expect("Calculating buddy_order has underflowed the usize");
                let block_size = PAGE_SIZE << buddy_order;

                unsafe {
                    let start_addr = area.as_ref().start_addr();
                    let buddy_addr = start_addr + block_size;

                    self.push_to_order(buddy_order, start_addr);
                    self.push_to_order(buddy_order, buddy_addr);
                }
            }
        }
        return Err(());
    }

    pub fn combine_free_buddies(&mut self, addr: usize, order: usize) {
        for current_order in MIN_ORDER..=order {
            let buddy_addr = addr ^ (PAGE_SIZE << current_order);

            if (buddy_addr ^ addr) == (PAGE_SIZE << current_order) 
                && self.list_areas[current_order].nr_free >= 2 
            {
                let new_addr = addr.min(buddy_addr);
                self.list_areas[current_order].head = None;
                self.list_areas[current_order].nr_free = 0;

                let node_ptr = new_addr as *mut FreeList;
                unsafe {
                    node_ptr.write_volatile(FreeList::new());
                    self.list_areas[current_order+1].push(NonNull::new_unchecked(node_ptr));
                }
            }
        }

        return;
    }

    fn push_to_order(&mut self, order: usize, addr: usize) {
        assert!(addr != 0, "Given address is NULL.");
        let node_ptr = addr as *mut FreeList;

        unsafe {
            node_ptr.write_volatile(FreeList::new());
            self.list_areas[order].push(NonNull::new_unchecked(node_ptr));
        }
    }

    fn size_align(layout: Layout) -> usize {
        let new_layout = layout
            .align_to(align_of::<FreeList>())
            .expect("adjusting alignment failed")
            .pad_to_align();

        let size_bytes = new_layout.size().max(size_of::<FreeList>());
        let size_in_pages = size_bytes.div_ceil(PAGE_SIZE);

        assert!(
            size_in_pages.ilog2() <= MAX_ORDER as u32,
            "Object is too large to allocate in set largest single block ((2^16)*16 = 1048576 bytes) in this allocator."
        );

        return size_in_pages;
    }
}

unsafe impl GlobalAlloc for Locked<BuddyAlloc> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = BuddyAlloc::size_align(layout);
        let mut allocator = self.lock();

        let alloc_order = size.ilog2() as usize;

        if allocator.split_area_to(alloc_order).is_err() {
            eprintln!("split area null");
            return null_mut();
        };

        let region = match allocator.list_areas[alloc_order].pop() {
            Some(f) => f,
            None => {
                eprintln!("region null");
                return null_mut();
            }
        };
        let alloc_start = unsafe { region.as_ref().start_addr() };

        return alloc_start as *mut u8;
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();

        let size = BuddyAlloc::size_align(layout);
        let dealloc_order = size.ilog2() as usize;

        unsafe { allocator.add_free_area(ptr as usize, dealloc_order) };
        allocator.combine_free_buddies(ptr as usize, dealloc_order);
    }
}
