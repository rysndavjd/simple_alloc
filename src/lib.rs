#![no_std]
#![allow(clippy::needless_return)] // I prefer specifying when a fn to return instead of the compiler trying to figure it out.

#[cfg(feature = "buddy_alloc")]
pub mod buddy_alloc;
#[cfg(feature = "bump_alloc")]
pub mod bump_alloc;
#[cfg(feature = "linked_list_alloc")]
pub mod linked_list_alloc;
pub(crate) mod common;
//pub mod linked_list_alloc;
pub use crate::common::{AllocInit, AllocState, BAllocator, BAllocatorError, align_up};

#[cfg(test)]
mod tests;
