#[cfg(feature = "buddy_alloc")]
pub mod buddy_alloc;
#[cfg(feature = "bump_alloc")]
pub mod bump_alloc;
#[cfg(feature = "linked_list_alloc")]
pub mod linked_list_alloc;
#[cfg(feature = "slab_alloc")]
pub mod slab_alloc;
