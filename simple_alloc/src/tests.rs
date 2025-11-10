extern crate alloc;

use core::{
    alloc::{GlobalAlloc, Layout},
    mem::MaybeUninit,
};

use alloc::vec::Vec;

use crate::{
    bump_alloc::{ConstBumpAlloc, LockedBumpAlloc},
    common::BAllocator,
};

#[test]
fn te() {
    let allocator = LockedBumpAlloc::new();
}

// #[test]
// fn bump_spin_boundary_conditions() {
//     const HEAP_SIZE: usize = 100;
//     static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

//     let allocator = Locked::new(BumpAlloc_spin::new());

//     unsafe {
//         allocator.lock().init(&raw mut HEAP_MEM as usize, HEAP_SIZE);

//         let layout = Layout::from_size_align(10, 1).unwrap();
//         let mut ptrs = Vec::new();

//         loop {
//             let ptr = allocator.alloc(layout);
//             if ptr.is_null() {
//                 break;
//             }
//             ptrs.push(ptr);
//         }

//         assert!(!ptrs.is_empty());

//         let ptr = allocator.alloc(layout);
//         assert!(ptr.is_null());
//     }
// }

// #[test]
// fn bump_lockless_boundary_conditions() {
//     const HEAP_SIZE: usize = 100;
//     static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

//     let mut allocator = BumpAlloc_lockless::new();

//     unsafe {
//         allocator.init(&raw mut HEAP_MEM as usize, HEAP_SIZE);

//         let layout = Layout::from_size_align(10, 1).unwrap();
//         let mut ptrs = Vec::new();

//         loop {
//             let ptr = allocator.alloc(layout);
//             if ptr.is_null() {
//                 break;
//             }
//             ptrs.push(ptr);
//         }

//         assert!(!ptrs.is_empty());

//         let ptr = allocator.alloc(layout);
//         assert!(ptr.is_null());
//     }
// }

// #[test]
// fn bump_const_boundary_conditions() {
//     const HEAP_SIZE: usize = 100;
//     let allocator = BumpAlloc_const::<HEAP_SIZE>::new();

//     unsafe {
//         let layout = Layout::from_size_align(10, 1).unwrap();
//         let mut ptrs = Vec::new();

//         loop {
//             let ptr = allocator.alloc(layout);
//             if ptr.is_null() {
//                 break;
//             }
//             ptrs.push(ptr);
//         }

//         assert!(!ptrs.is_empty());

//         let ptr = allocator.alloc(layout);
//         assert!(ptr.is_null());
//     }
// }

// #[test]
// fn linked_list_spin_combine_free_regions() {
//     const HEAP_SIZE: usize = 64;
//     static mut HEAP_MEM: LinkedListHeap<HEAP_SIZE> = LinkedListHeap::new();

//     let allocator = Locked::new(LinkedListAlloc_spin::new());

//     unsafe {
//         allocator.lock().init(&raw mut HEAP_MEM);

//         let layout_u32 = Layout::new::<u32>();

//         // println!("Initialized empty heap");
//         // print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
//         // println!();

//         let ptr1 = allocator.alloc(layout_u32) as *mut u32;
//         let ptr2 = allocator.alloc(layout_u32) as *mut u32;
//         let ptr3 = allocator.alloc(layout_u32) as *mut u32;
//         let ptr4 = allocator.alloc(layout_u32) as *mut u32;

//         assert!(!ptr1.is_null());
//         assert!(!ptr2.is_null());
//         assert!(!ptr3.is_null());
//         assert!(!ptr4.is_null());

//         // println!("Heap allocated with 4 u32 numbers");
//         // print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
//         // println!();

//         *ptr1 = u32::MAX;
//         *ptr2 = u32::MAX;
//         *ptr3 = u32::MAX;
//         *ptr4 = u32::MAX;

//         // println!("u32 numbers are set to u32::max");
//         // print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
//         // println!();

//         assert_eq!(*ptr4, u32::MAX);
//         assert_eq!(*ptr3, u32::MAX);
//         assert_eq!(*ptr2, u32::MAX);
//         assert_eq!(*ptr1, u32::MAX);

//         allocator.dealloc(ptr4 as *mut u8, layout_u32);
//         allocator.dealloc(ptr3 as *mut u8, layout_u32);
//         allocator.dealloc(ptr2 as *mut u8, layout_u32);
//         allocator.dealloc(ptr1 as *mut u8, layout_u32);

//         // println!("u32 numbers are deallocated");
//         // print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
//         // println!();

//         let layout_u64 = Layout::new::<u64>();

//         let ptr5 = allocator.alloc(layout_u64) as *mut u64;
//         let ptr6 = allocator.alloc(layout_u64) as *mut u64;
//         let ptr7 = allocator.alloc(layout_u64) as *mut u64;
//         let ptr8 = allocator.alloc(layout_u64) as *mut u64;

//         assert!(!ptr5.is_null());
//         assert!(!ptr6.is_null());
//         assert!(!ptr7.is_null());
//         assert!(!ptr8.is_null());

//         // println!("Heap allocated with 4 u64 numbers");
//         // print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
//         // println!();

//         *ptr5 = u64::MAX;
//         *ptr6 = u64::MAX;
//         *ptr7 = u64::MAX;
//         *ptr8 = u64::MAX;

//         // println!("u64 numbers are set to u64::max");
//         // print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
//         // println!();

//         assert_eq!(*ptr8, u64::MAX);
//         assert_eq!(*ptr7, u64::MAX);
//         assert_eq!(*ptr6, u64::MAX);
//         assert_eq!(*ptr5, u64::MAX);

//         allocator.dealloc(ptr8 as *mut u8, layout_u64);
//         allocator.dealloc(ptr7 as *mut u8, layout_u64);
//         allocator.dealloc(ptr6 as *mut u8, layout_u64);
//         allocator.dealloc(ptr5 as *mut u8, layout_u64);

//         // println!("u64 numbers are deallocated");
//         // print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
//         // println!();
//     }
// }

// #[test]
// fn linked_list_spin_boundary_conditions() {
//     const HEAP_SIZE: usize = 64;
//     static mut HEAP_MEM: LinkedListHeap<HEAP_SIZE> = LinkedListHeap::new();

//     let allocator = Locked::new(LinkedListAlloc_spin::new());

//     unsafe {
//         allocator.lock().init(&raw mut HEAP_MEM);

//         let layout = Layout::from_size_align(10, 1).unwrap();
//         let mut ptrs = Vec::new();

//         loop {
//             let ptr = allocator.alloc(layout);
//             if ptr.is_null() {
//                 break;
//             }
//             ptrs.push(ptr);
//         }

//         assert!(!ptrs.is_empty());

//         let ptr = allocator.alloc(layout);
//         assert!(ptr.is_null());
//     }
// }

// #[test]
// fn buddy_spin_alloc() {
//     #[repr(align(8))]
//     pub struct Heap<const HEAP_SIZE: usize>(pub [MaybeUninit<u8>; HEAP_SIZE]);

//     const HEAP_SIZE: usize = 512;
//     static mut HEAP_MEM: Heap<HEAP_SIZE> = Heap([MaybeUninit::uninit(); HEAP_SIZE]);

//     let allocator = Locked::new(BuddyAlloc_spin::new());

//     unsafe {
//         allocator
//             .lock()
//             .init_with_ptr(&raw mut HEAP_MEM as usize, HEAP_SIZE);

//         // println!("{:?}", *allocator.lock());
//         // print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
//         // println!();

//         let layout_u8_16 = Layout::new::<[u8; 11]>();

//         let ptr1_u8_16 = allocator.alloc(layout_u8_16) as *mut [u8; 11];
//         ptr1_u8_16.write([0xFF_u8; 11]);

//         // println!("{:?}", *allocator.lock());
//         // print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
//         // println!();

//         allocator.dealloc(ptr1_u8_16 as *mut u8, layout_u8_16);

//         // println!("{:?}", *allocator.lock());
//         // print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
//         // println!();
//     }
// }
