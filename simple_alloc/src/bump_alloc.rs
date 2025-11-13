use conquer_once::spin::OnceCell;
use spin::Mutex;

use crate::common::Alloc;

mod bconst;
mod locked;
mod lockless;

pub use crate::bump_alloc::bconst::ConstBump;
pub use crate::bump_alloc::locked::LockedBump;
pub use crate::bump_alloc::lockless::LocklessBump;

pub type LockedBumpAlloc = Alloc<Mutex<LockedBump>>;
pub type LocklessBumpAlloc = Alloc<OnceCell<LocklessBump>>;
pub type ConstBumpAlloc<const S: usize> = Alloc<ConstBump<S>>;
