use spin::Mutex;

use crate::common::Alloc;

mod bconst;
mod locked;
mod lockless;

pub use crate::buddy_alloc::locked::LockedBuddy;

pub type LockedBuddyAlloc<'a> = Alloc<Mutex<LockedBuddy<'a>>>;
