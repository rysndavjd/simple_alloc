#![cfg_attr(feature = "allocator-api", feature(allocator_api))]

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(all(not(feature = "std"), not(test)))]
extern crate core as std;

mod bump_alloc;
mod common;
mod ralloc;

#[cfg(feature = "bump_alloc")]
pub use bump_alloc::BumpAlloc;

pub use crate::common::{Allocations, Bytes, Initialization};
