use spin::mutex::Mutex;

use crate::{
    common::AllocatorError,
    ralloc::buddy::Buddy,
    std::{alloc::Layout, ptr::NonNull},
};

mod buddy;

pub const PAGE_SIZE: usize = 4096;
pub const PAGE_SHIFT: usize = 12;

#[repr(usize)]
enum SizeClass {
    Eight = 8,
    Sixteen = 16,
    ThirtyTwo = 32,
    SixtyFour = 64,
    HundredTwentyEight = 128,
    TwoHundredFiftySix = 256,
    FiveHundredTwelve = 512,
    ThousandTwentyFour = 1024,
}

impl SizeClass {
    /// Number of classes that has a bin for allocations
    const NR_BINNED: usize = 8;
}

enum AllocClass {
    Binned(SizeClass),
    /// Number of pages needed for allocation
    Large(usize),
}

impl AllocClass {
    fn pages_needed(size: usize) -> usize {
        size.div_ceil(PAGE_SIZE)
    }
}

impl From<usize> for AllocClass {
    fn from(size: usize) -> Self {
        match size {
            1..=8 => Self::Binned(SizeClass::Eight),
            9..=16 => Self::Binned(SizeClass::Sixteen),
            17..=32 => Self::Binned(SizeClass::ThirtyTwo),
            33..=64 => Self::Binned(SizeClass::SixtyFour),
            65..=128 => Self::Binned(SizeClass::HundredTwentyEight),
            129..=256 => Self::Binned(SizeClass::TwoHundredFiftySix),
            257..=512 => Self::Binned(SizeClass::FiveHundredTwelve),
            513..=1024 => Self::Binned(SizeClass::ThousandTwentyFour),
            _ => Self::Large(AllocClass::pages_needed(size)),
        }
    }
}

struct FreeSlot {
    next: Option<NonNull<FreeSlot>>,
}

/// A free page is a 4096 byte memory region that
/// how many free allocations are available and a
/// free linked list doing lazy initialization for
/// each allocated object.
struct FreePage {
    next: Option<NonNull<FreePage>>,
    prev: Option<NonNull<FreePage>>,
    free_allocations: usize,
    slots: Option<NonNull<FreeSlot>>,
}

/// A bin is a list of pages that contain a single size class of objects.
struct Bin {
    class: SizeClass,
    head: Option<NonNull<FreePage>>,
    tail: Option<NonNull<FreePage>>,
}

struct Ralloc<const NR_ORDER: usize> {
    page_alloc: Mutex<Buddy<NR_ORDER>>,
    bins: [Mutex<Bin>; SizeClass::NR_BINNED],
}

impl<const NR_ORDER: usize> Ralloc<NR_ORDER> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocatorError> {
        todo!()
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        todo!()
    }
}
