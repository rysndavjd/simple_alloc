use core::{
    alloc::Layout,
    fmt::{Debug, Formatter, Result as FmtResult},
    mem::{align_of, size_of},
    ptr::{NonNull, null_mut},
    sync::atomic::{AtomicBool, Ordering},
};

#[cfg(debug_assertions)]
use log::{debug, error, trace};
use spin::Mutex;

use crate::common::{
    ALLOCATOR_ALREADY_INITIALIZED, ALLOCATOR_UNINITIALIZED, Alloc, AllocInit, BAllocator,
    BAllocatorError, HEAP_END_OVERFLOWED, HEAP_NOT_POWER_TWO, HEAP_SIZE_ZERO, HEAP_START_NULL, OOM,
    align_up,
};

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

pub const MIN_ALLOCATION_SIZE: usize = 8;
pub const MIN_ORDER: usize = 0;
pub const MAX_ORDER: usize = 32;
pub const NR_MAX_ORDER: usize = MAX_ORDER + 1;

pub struct LockedBuddy {
    base: *mut u8,
    size: usize,
    list_areas: [FreeArea; NR_MAX_ORDER],
}

impl Debug for Alloc<Mutex<LockedBuddy>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let alloc = self.alloc.lock();
        writeln!(f, "LockedBuddy {{")?;
        writeln!(f, "    base: {:?}", alloc.base)?;
        writeln!(f, "    size: {}", alloc.size)?;
        writeln!(f, "    list_areas: [")?;
        for (i, v) in alloc.list_areas.iter().enumerate() {
            writeln!(f, "    {}: {:?}", i, v)?;
        }
        writeln!(f, "]}}")
    }
}

impl LockedBuddy {
    const fn new() -> LockedBuddy {
        LockedBuddy {
            base: null_mut(),
            size: 0,
            list_areas: [const { FreeArea::new() }; NR_MAX_ORDER],
        }
    }

    unsafe fn add_free_area(&mut self, addr: usize, order: usize) {
        assert!(
            addr != 0,
            "add_free_area: Given free area has a NULL address pointer."
        );
        assert_eq!(align_up(addr, align_of::<FreeList>()), addr);

        let mut new_item = FreeList::new();
        new_item.next = self.list_areas[order].head;

        let item_ptr = addr as *mut FreeList;

        unsafe {
            #[cfg(debug_assertions)]
            trace!(
                "Wrote item: {:?}, at Addr: {:#X}",
                new_item, item_ptr as usize
            );
            item_ptr.write_volatile(new_item);
            self.list_areas[order].head = NonNull::new(item_ptr);
            self.list_areas[order].nr_free += 1;
        }
    }

    fn split_area_to(&mut self, target_order: usize) -> Result<(), BAllocatorError> {
        let source_order = (target_order..NR_MAX_ORDER)
            .find(|&order| self.list_areas[order].nr_free > 0)
            .ok_or(BAllocatorError::Oom(None))?;

        for current_order in (target_order..=source_order).rev() {
            if self.list_areas[current_order].nr_free > 0 {
                if current_order == target_order {
                    return Ok(());
                }
                let area = self.list_areas[current_order]
                    .pop()
                    .ok_or(BAllocatorError::Oom(None))?;

                let buddy_order = current_order
                    .checked_sub(1) // This should normally never underflow but checking just in case.
                    .ok_or(BAllocatorError::Underflowed)?;
                let block_size = MIN_ALLOCATION_SIZE << buddy_order;

                unsafe {
                    let start_addr = area.as_ref().start_addr();
                    let buddy_addr = start_addr + block_size;

                    self.push_to_order(buddy_order, start_addr);
                    self.push_to_order(buddy_order, buddy_addr);
                    #[cfg(debug_assertions)]
                    trace!(
                        "Pushed to order: {}, start_addr: {:#X}, buddy_addr: {:#X}",
                        buddy_order, start_addr, buddy_addr
                    );
                }
            }
        }
        return Err(BAllocatorError::Oom(None));
    }

    fn combine_free_buddies(&mut self, addr: usize) {
        assert!(addr != 0, "combine_free_buddies: Given address is NULL");
        for current_order in MIN_ORDER..=MAX_ORDER {
            let buddy_addr = addr ^ (MIN_ALLOCATION_SIZE << current_order);

            if (buddy_addr ^ addr) == (MIN_ALLOCATION_SIZE << current_order)
                && self.list_areas[current_order].nr_free >= 2
            {
                let new_addr = addr.min(buddy_addr);
                self.list_areas[current_order].head = None;
                self.list_areas[current_order].nr_free = 0;

                let node_ptr = new_addr as *mut FreeList;
                unsafe {
                    node_ptr.write_volatile(FreeList::new());
                    self.list_areas[current_order + 1].push(NonNull::new_unchecked(node_ptr));
                }
            }
        }
    }

    fn push_to_order(&mut self, order: usize, addr: usize) {
        assert!(addr != 0, "push_to_order: Given address is NULL.");
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
        let size_min_allocation = size_bytes.div_ceil(MIN_ALLOCATION_SIZE);

        assert!(
            size_min_allocation.ilog2() <= MAX_ORDER as u32,
            "Object is too large to allocate in set largest single block in this allocator."
        );

        return size_min_allocation;
    }
}

unsafe impl BAllocator for Mutex<LockedBuddy> {
    fn try_allocate(&self, layout: Layout) -> Result<NonNull<u8>, BAllocatorError> {
        assert!(self.is_initialized(), "{ALLOCATOR_UNINITIALIZED}");

        let size = LockedBuddy::size_align(layout);
        let mut allocator = self.lock();

        let alloc_order = size.ilog2() as usize;

        allocator.split_area_to(alloc_order)?;

        let region = match allocator.list_areas[alloc_order].pop() {
            Some(f) => f,
            None => {
                #[cfg(debug_assertions)]
                error!("{}", OOM);
                return Err(BAllocatorError::Oom(Some(layout)));
            }
        };
        let alloc_start = region.as_ptr() as *mut u8;

        #[cfg(debug_assertions)]
        debug!(
            "Allocated object \"{:X}\"; layout: {layout:?}",
            alloc_start as usize
        );
        return Ok(unsafe { NonNull::new_unchecked(alloc_start) });
    }

    fn try_deallocate(&self, ptr: NonNull<u8>, layout: Layout) -> Result<(), BAllocatorError> {
        assert!(self.is_initialized(), "{ALLOCATOR_UNINITIALIZED}");

        let mut allocator = self.lock();

        let size = LockedBuddy::size_align(layout);
        let dealloc_order = size.ilog2() as usize;

        unsafe { allocator.add_free_area(ptr.as_ptr() as usize, dealloc_order) };
        allocator.combine_free_buddies(ptr.as_ptr() as usize);

        #[cfg(debug_assertions)]
        debug!(
            "Deallocated object \"{:X}\"; layout: {layout:?}",
            ptr.as_ptr() as usize
        );
        return Ok(());
    }
}

unsafe impl Sync for Alloc<Mutex<LockedBuddy>> {}
unsafe impl Send for Alloc<Mutex<LockedBuddy>> {}

impl Alloc<Mutex<LockedBuddy>> {
    pub const fn new() -> Self {
        Alloc {
            alloc: Mutex::new(LockedBuddy::new()),
        }
    }
}

impl Default for Alloc<Mutex<LockedBuddy>> {
    fn default() -> Self {
        Self::new()
    }
}

static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initializes the buddy allocator making it available for use.
///
/// See [`AllocInit::init`] for safety requirements.
impl AllocInit for Mutex<LockedBuddy> {
    fn is_initialized(&self) -> bool {
        return INITIALIZED.load(Ordering::Acquire);
    }

    unsafe fn init(&self, start: usize, size: usize) {
        if INITIALIZED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            panic!("{ALLOCATOR_ALREADY_INITIALIZED}");
        }

        assert!(start != 0, "{HEAP_START_NULL}");
        assert!(size > 0, "{HEAP_SIZE_ZERO}");
        assert!(start + size < usize::MAX, "{HEAP_END_OVERFLOWED}");
        assert!(size.is_power_of_two(), "{HEAP_NOT_POWER_TWO}");
        assert_eq!(
            align_up(start, align_of::<FreeList>()),
            start,
            "Given start is not 8 byte aligned"
        );

        let mut alloc = self.lock();

        alloc.base = start as *mut u8;
        alloc.size = size;

        unsafe {
            alloc.add_free_area(start, size.div_ceil(MIN_ALLOCATION_SIZE).ilog2() as usize);
        }

        #[cfg(debug_assertions)]
        debug!("Initialized locked buddy alloc; start: {start:#X}, size: {size}");
    }
}
