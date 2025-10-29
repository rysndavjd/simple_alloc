#![allow(clippy::needless_return)] // I prefer specifying when a fn to return instead of the compiler trying to figure it out.
#![allow(clippy::new_without_default)] // Annoying

pub mod common;
#[cfg(feature = "const_able")]
mod const_able;
#[cfg(feature = "lockless")]
mod lockless;
#[cfg(feature = "spin")]
mod spin_lock;

#[cfg(test)]
mod tests;
