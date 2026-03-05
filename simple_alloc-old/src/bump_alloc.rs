use conquer_once::spin::OnceCell;
use spin::Mutex;

use crate::common::Alloc;

mod locked;
mod lockless;

use crate::bump_alloc::locked::LockedBump;
use crate::bump_alloc::lockless::LocklessBump;

pub type LockedBumpAlloc = Alloc<Mutex<LockedBump>>;
pub type LocklessBumpAlloc = Alloc<OnceCell<LocklessBump>>;