# Simple_alloc
A collection of simple, pure Rust memory allocators for `no_std` and embedded environments. Prioritizing simplicity and minimal dependencies.


| Memory Allocator | Status | Const Variant | Atomic Variant | Blocking Variant | 
|------------------|--------|---------------|----------------|--------------|
| [Bump Alloc](https://os.phil-opp.com/allocator-designs/#bump-allocator) | Works | Yes | Yes | Yes |
| [Linked List Alloc](https://os.phil-opp.com/allocator-designs/#linked-list-allocator) | Inprogress | No | No | No |
| [Buddy Alloc](https://en.wikipedia.org/wiki/Buddy_memory_allocation) |  Works | No | No | Yes |
| [Slab Alloc](https://en.wikipedia.org/wiki/Slab_allocation) | Inprogress | No | No | No |

### Status Definitions

- **Inprogress**: Not fully implemented or finished.
- **Works**: Functionally implemented, not tested fully, odd edge cases may occur.  
- **Complete**: Fully implemented and tested.

### Variants Definitions

- **Blocking**: Initialized at runtime time utilizing an external heap. Uses [spinlocks](https://en.wikipedia.org/wiki/Spinlock) internally for its logic.
- **Atomic**: Initialized at runtime time utilizing an external heap. Uses [atomics](https://doc.rust-lang.org/core/sync/atomic/index.html) internally for its logic.
- **Const**: Initialized at compile time utilizing an internal heap. Uses [atomics](https://doc.rust-lang.org/core/sync/atomic/index.html) internally for its logic as it is usually more performant.

## Acknowledgments

- [Philipp Oppermann's "Writing an OS in Rust"](https://os.phil-opp.com/) - Comprehensive explanations of allocator designs with bump and linked list allocators adapted from Philipp Oppermann's implementations with small tweaks.

## License

This project is dual-licensed under either:

- [MIT License](LICENSE-MIT), or
- [Apache License 2.0](LICENSE-APACHE)

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions. 