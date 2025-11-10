# Simple_alloc
A collection of simple, pure Rust memory allocators for `no_std` and embedded environments. Prioritizing simplicity and minimal dependencies, each allocator implements the [`GlobalAlloc`](https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html) trait, enabling heap allocation in `no_std` environments through Rust's [`alloc`](https://doc.rust-lang.org/alloc/alloc/index.html) crate.


| Memory Allocator | Status | Const Variant | 
|------------------|--------|---------------|
| [Bump Alloc](https://os.phil-opp.com/allocator-designs/#bump-allocator) | Works | yes |
| [Linked List Alloc](https://os.phil-opp.com/allocator-designs/#linked-list-allocator) | Inprogress | no |
| [Buddy Alloc](https://en.wikipedia.org/wiki/Buddy_memory_allocation) |  Inprogress | no |
| [Slab Alloc](https://en.wikipedia.org/wiki/Slab_allocation) | Inprogress | no |

### Status Definitions

- **Inprogress**: Not fully implemented or finished.
- **Works**: Functionally implemented, not tested fully, odd edge cases may occur.  
- **Complete**: Fully implemented and tested.

### Const Variant

Shows if a memory allocator is available as a const variant to be initialized at compile time
instead of runtime.

## Acknowledgments

- [Philipp Oppermann's "Writing an OS in Rust"](https://os.phil-opp.com/) - Comprehensive explanations of allocator designs with bump and linked list allocators adapted from Philipp Oppermann's implementations with small tweaks.

## License

This project is dual-licensed under either:

- [MIT License](LICENSE-MIT), or
- [Apache License 2.0](LICENSE-APACHE)

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions. 