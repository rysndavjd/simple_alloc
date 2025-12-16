use spin::Mutex;

use crate::common::Alloc;

mod bconst;
mod locked;
mod lockless;

use crate::buddy_alloc::locked::LockedBuddy;

pub type LockedBuddyAlloc = Alloc<Mutex<LockedBuddy>>;
