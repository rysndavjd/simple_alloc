#![no_std]
#![allow(clippy::needless_return)] // I prefer specifying when a fn to return instead of the compiler trying to figure it out.
#![allow(clippy::new_without_default)] // Annoying
//#![feature(allocator_api)]

#[cfg(feature = "bump_alloc")]
pub mod bump_alloc;
pub(crate) mod common;
pub use crate::common::BAllocator;

#[cfg(test)]
mod tests;
