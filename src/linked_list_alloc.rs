use spin::Mutex;

use crate::common::Alloc;

mod bconst;
mod locked;
mod lockless;

use crate::linked_list_alloc::locked::LockedLinkedList;

pub type LockedLinkedListAlloc = Alloc<Mutex<LockedLinkedList>>;
