use spin::mutex::Mutex;

use crate::{ralloc::buddy::Buddy, std::ptr::NonNull};

mod buddy;
mod utils;

pub const PAGE_SIZE: usize = 4096;

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
    const NUM_BINNED: usize = 8;
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

/// A page is a 4096 byte memory region with metadata attached to it.
struct Page {
    allocations: usize,
    next: Option<NonNull<Page>>,
    prev: Option<NonNull<Page>>,
    slots: Option<NonNull<FreeSlot>>,
}

/// A bin is a list of pages that contain a single size class of objects.
struct Bin {
    class: SizeClass,
    head: Option<NonNull<Page>>,
    tail: Option<NonNull<Page>>,
}

struct Ralloc<const MAX_ORDER: usize> {
    page_alloc: Buddy<MAX_ORDER>,
    bins: [Mutex<Bin>; SizeClass::NUM_BINNED],
}

// impl Ralloc {}
