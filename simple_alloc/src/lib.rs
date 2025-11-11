#![no_std]
#![allow(clippy::needless_return)] // I prefer specifying when a fn to return instead of the compiler trying to figure it out.
//#![feature(allocator_api)]

pub mod buddy_alloc;
#[cfg(feature = "bump_alloc")]
pub mod bump_alloc;
pub(crate) mod common;
//pub mod linked_list_alloc;
pub use crate::common::{BAllocator, BAllocatorError, ConstAlloc, LockedAlloc, LocklessAlloc};

#[cfg(test)]
mod tests;
