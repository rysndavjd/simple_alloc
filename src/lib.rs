#![allow(clippy::needless_return)] // I prefer specifying when a fn to return instead of the compiler trying to figure it out.
#![allow(clippy::new_without_default)] // Annoying

#[cfg(feature = "buddy_alloc")]
pub mod buddy_alloc;
#[cfg(feature = "bump_alloc")]
pub mod bump_alloc;
pub mod common;
#[cfg(feature = "const_bump_alloc")]
pub mod const_bump_alloc;
#[cfg(feature = "linked_list_alloc")]
pub mod linked_list_alloc;
#[cfg(feature = "slab_alloc")]
pub mod slab_alloc;
#[cfg(test)]
mod tests;
