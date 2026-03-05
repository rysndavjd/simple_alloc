#![no_std]
#![allow(clippy::needless_return)] // I prefer specifying when a fn to return instead of the compiler trying to figure it out.
#![cfg_attr(feature = "allocator-api", feature(allocator_api))]

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(all(not(feature = "std"), not(test)))]
extern crate core as std;

extern crate alloc;

#[cfg(feature = "buddy_alloc")]
pub mod buddy_alloc;
#[cfg(feature = "bump_alloc")]
pub mod bump_alloc;
pub(crate) mod common;
#[cfg(feature = "linked_list_alloc")]
//pub mod linked_list_alloc;
pub use crate::common::{AllocInit, Allocations, BAllocator, BAllocatorError, align_up};

#[cfg(test)]
mod tests;
