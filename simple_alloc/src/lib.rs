#![allow(clippy::needless_return)] // I prefer specifying when a fn to return instead of the compiler trying to figure it out.
#![allow(clippy::new_without_default)] // Annoying
#![feature(allocator_api)]

pub mod common;
#[cfg(feature = "const_able")]
pub mod const_able;
#[cfg(feature = "lockless")]
pub mod lockless;
#[cfg(feature = "spin")]
pub mod spin_lock;

#[cfg(test)]
mod tests;
