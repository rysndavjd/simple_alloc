use conquer_once::spin::OnceCell;

use crate::{
    AllocInit,
    common::{
        ALLOCATOR_ALREADY_INITIALIZED, ALLOCATOR_UNINITIALIZED, Alloc, BAllocator, BAllocatorError,
        HEAP_END_OVERFLOWED, HEAP_NOT_POWER_TWO, HEAP_SIZE_ZERO, HEAP_START_NULL, align_up,
    },
    std::{
        alloc::Layout,
        fmt::{Debug, Formatter, Result as FmtResult},
        ptr::{NonNull, null_mut, slice_from_raw_parts_mut},
        sync::atomic::{AtomicBool, AtomicPtr, AtomicU8, Ordering},
    },
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
    pub head: AtomicPtr<FreeList>,
    pub nr_free: AtomicU8,
}

impl FreeArea {
    const fn new() -> FreeArea {
        FreeArea {
            head: AtomicPtr::new(null_mut()),
            nr_free: AtomicU8::new(0),
        }
    }

    fn push(&self, mut value: NonNull<FreeList>) {
        loop {
            let head = self.head.load(Ordering::Acquire);

            unsafe {
                value.as_mut().next = if head.is_null() {
                    None
                } else {
                    Some(NonNull::new_unchecked(head))
                };
            }

            if self
                .head
                .compare_exchange(head, value.as_ptr(), Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                self.nr_free.fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
    }

    fn pop(&self) -> Option<NonNull<FreeList>> {
        loop {
            let head = self.head.load(Ordering::Acquire);

            if head.is_null() {
                return None;
            } else {
                let new_head = match unsafe { (*head).next } {
                    Some(ptr) => ptr.as_ptr(),
                    None => null_mut(),
                };

                if self
                    .head
                    .compare_exchange(head, new_head, Ordering::AcqRel, Ordering::Relaxed)
                    .is_ok()
                {
                    self.nr_free.fetch_sub(1, Ordering::Relaxed);
                    break NonNull::new(head);
                }
            }
        }
    }
}

pub const MIN_ALLOCATION_SIZE: usize = 8;
pub const MIN_ORDER: usize = 0;
pub const MAX_ORDER: usize = 32;
pub const NR_MAX_ORDER: usize = MAX_ORDER + 1;

pub struct LocklessBuddy {
    base: *mut u8,
    size: usize,
    list_areas: [FreeArea; NR_MAX_ORDER],
}

// impl Debug for Alloc<OnceCell<LocklessBuddy>> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
//         let alloc = self.alloc.lock();
//         writeln!(f, "LocklessBuddy {{")?;
//         writeln!(f, "    base: {:?}", alloc.base)?;
//         writeln!(f, "    size: {}", alloc.size)?;
//         writeln!(f, "    list_areas: [")?;
//         for (i, v) in alloc.list_areas.iter().enumerate() {
//             writeln!(f, "    {}: {:?}", i, v)?;
//         }
//         writeln!(f, "]}}")
//     }
// }

impl LocklessBuddy {
    const fn empty() -> LocklessBuddy {
        Self {
            base: null_mut(),
            size: 0,
            list_areas: [const { FreeArea::new() }; NR_MAX_ORDER],
        }
    }

    unsafe fn add_free_area(&self, addr: usize, order: usize) {
        assert!(
            addr != 0,
            "add_free_area: Given free area has a NULL address pointer."
        );
        assert_eq!(align_up(addr, align_of::<FreeList>()), addr);

        let node_ptr = addr as *mut FreeList;

        loop {
            let head = self.list_areas[order].head.load(Ordering::Acquire);
            unsafe {
                (*node_ptr).next = match head.as_mut() {
                    Some(ptr) => NonNull::new(ptr),
                    None => None,
                };
            }

            if self.list_areas[order]
                .head
                .compare_exchange(head, node_ptr, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                self.list_areas[order]
                    .nr_free
                    .fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
    }

    fn split_area_to(&self, target_order: usize) -> Result<(), BAllocatorError> {
        let source_order = (target_order..NR_MAX_ORDER)
            .find(|&order| self.list_areas[order].nr_free.load(Ordering::Acquire) > 0)
            .ok_or(BAllocatorError::Oom(None))?;

        for current_order in (target_order..=source_order).rev() {
            if let Some(area) = self.list_areas[current_order].pop() {
                if current_order == target_order {
                    self.list_areas[current_order].push(area);
                    return Ok(());
                }

                let buddy_order = current_order
                    .checked_sub(1)
                    .ok_or(BAllocatorError::Underflowed)?;
                let block_size = MIN_ALLOCATION_SIZE << buddy_order;

                unsafe {
                    let start_addr = area.as_ref().start_addr();
                    let buddy_addr = start_addr + block_size;

                    self.push_to_order(buddy_order, start_addr);
                    self.push_to_order(buddy_order, buddy_addr);
                }
            }
        }
        return Err(BAllocatorError::Oom(None));
    }

    fn combine_free_buddies(&self, addr: usize) {
        assert!(addr != 0, "combine_free_buddies: Given address is NULL");

        todo!()
    }

    fn push_to_order(&self, order: usize, addr: usize) {
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
            "Object is too large to allocate in set largest single block in this allocator"
        );

        return size_min_allocation;
    }
}

unsafe impl BAllocator for OnceCell<LocklessBuddy> {
    fn try_allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, BAllocatorError> {
        assert!(self.is_initialized(), "{ALLOCATOR_UNINITIALIZED}");

        let size = LocklessBuddy::size_align(layout);
        let allocator = unsafe { self.get_unchecked() };

        let alloc_order = size.ilog2() as usize;

        allocator.split_area_to(alloc_order)?;

        let region = match allocator.list_areas[alloc_order].pop() {
            Some(f) => f,
            None => {
                // #[cfg(debug_assertions)]
                // error!("{}", OOM);
                return Err(BAllocatorError::Oom(Some(layout)));
            }
        };
        let alloc_start = region.as_ptr() as *mut u8;

        // #[cfg(debug_assertions)]
        // debug!(
        //     "Allocated object \"{:X}\"; layout: {layout:?}",
        //     alloc_start as usize
        // );
        return Ok(unsafe {
            NonNull::new_unchecked(slice_from_raw_parts_mut(alloc_start, layout.size()))
        });
    }

    fn try_deallocate(&self, ptr: NonNull<[u8]>, layout: Layout) -> Result<(), BAllocatorError> {
        assert!(self.is_initialized(), "{ALLOCATOR_UNINITIALIZED}");

        let allocator = unsafe { self.get_unchecked() };

        let size = LocklessBuddy::size_align(layout);
        let dealloc_order = size.ilog2() as usize;

        unsafe { allocator.add_free_area(ptr.as_ptr() as *mut u8 as usize, dealloc_order) };
        //allocator.combine_free_buddies(ptr.as_ptr() as usize);

        // #[cfg(debug_assertions)]
        // debug!(
        //     "Deallocated object \"{:X}\"; layout: {layout:?}",
        //     ptr.as_ptr() as usize
        // );
        return Ok(());
    }
}

unsafe impl Sync for Alloc<OnceCell<LocklessBuddy>> {}
unsafe impl Send for Alloc<OnceCell<LocklessBuddy>> {}

static INITIALIZED: AtomicBool = AtomicBool::new(false);

impl AllocInit for OnceCell<LocklessBuddy> {
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

        self.init_once(|| {
            let mut buddy = LocklessBuddy::empty();

            buddy.base = start as *mut u8;
            buddy.size = size;

            unsafe {
                buddy.add_free_area(start, size.div_ceil(MIN_ALLOCATION_SIZE).ilog2() as usize);
            }

            return buddy;
        });

        // #[cfg(debug_assertions)]
        // debug!("Initialized lockless buddy alloc; start: {start:#X}, size: {size}");
    }
}
