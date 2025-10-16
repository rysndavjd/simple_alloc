#![allow(clippy::needless_return)]

#[cfg(feature = "buddy_alloc")]
mod buddy_alloc;
#[cfg(feature = "bump_alloc")]
mod bump_alloc;
mod common;
#[cfg(feature = "linked_list_alloc")]
mod linked_list_alloc;
#[cfg(feature = "slab_alloc")]
mod slab_alloc;

#[cfg(test)]
mod tests;

#[cfg(feature = "buddy_alloc")]
pub use buddy_alloc::{BuddyAlloc, BuddyHeap};
#[cfg(feature = "bump_alloc")]
pub use bump_alloc::{BumpAlloc, BumpHeap};
pub use common::Locked;
#[cfg(feature = "linked_list_alloc")]
pub use linked_list_alloc::{LinkedListAlloc, LinkedListHeap};