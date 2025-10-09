extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::mem::MaybeUninit;

use crate::common::print_heap_dump;
use crate::{
    BuddyAlloc, BumpAlloc, BumpHeap, LinkedListAlloc, LinkedListHeap, common::Locked,
};

#[test]
fn bump_boundary_conditions() {
    const HEAP_SIZE: usize = 100;
    static mut HEAP_MEM: BumpHeap<HEAP_SIZE> = BumpHeap::new();

    let allocator = Locked::new(BumpAlloc::new());

    unsafe {
        allocator.lock().init::<HEAP_SIZE>(&raw mut HEAP_MEM);

        let layout = Layout::from_size_align(10, 1).unwrap();
        let mut ptrs = Vec::new();

        loop {
            let ptr = allocator.alloc(layout);
            if ptr.is_null() {
                break;
            }
            ptrs.push(ptr);
        }

        assert!(!ptrs.is_empty());

        let ptr = allocator.alloc(layout);
        assert!(ptr.is_null());
    }
}

#[test]
fn linked_list_combine_free_regions() {
    const HEAP_SIZE: usize = 64;
    static mut HEAP_MEM: LinkedListHeap<HEAP_SIZE> = LinkedListHeap::new();

    let allocator = Locked::new(LinkedListAlloc::new());

    unsafe {
        allocator.lock().init(&raw mut HEAP_MEM);

        let layout_u32 = Layout::new::<u32>();

        println!("Initialized empty heap");
        print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
        println!();

        let ptr1 = allocator.alloc(layout_u32) as *mut u32;
        let ptr2 = allocator.alloc(layout_u32) as *mut u32;
        let ptr3 = allocator.alloc(layout_u32) as *mut u32;
        let ptr4 = allocator.alloc(layout_u32) as *mut u32;

        assert!(!ptr1.is_null());
        assert!(!ptr2.is_null());
        assert!(!ptr3.is_null());
        assert!(!ptr4.is_null());

        println!("Heap allocated with 4 u32 numbers");
        print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
        println!();

        *ptr1 = u32::MAX;
        *ptr2 = u32::MAX;
        *ptr3 = u32::MAX;
        *ptr4 = u32::MAX;

        println!("u32 numbers are set to u32::max");
        print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
        println!();

        assert_eq!(*ptr4, u32::MAX);
        assert_eq!(*ptr3, u32::MAX);
        assert_eq!(*ptr2, u32::MAX);
        assert_eq!(*ptr1, u32::MAX);

        allocator.dealloc(ptr4 as *mut u8, layout_u32);
        allocator.dealloc(ptr3 as *mut u8, layout_u32);
        allocator.dealloc(ptr2 as *mut u8, layout_u32);
        allocator.dealloc(ptr1 as *mut u8, layout_u32);

        let layout_u64 = Layout::new::<u64>();

        println!("u32 numbers are deallocated");
        print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
        println!();

        let ptr5 = allocator.alloc(layout_u64) as *mut u64;
        let ptr6 = allocator.alloc(layout_u64) as *mut u64;
        let ptr7 = allocator.alloc(layout_u64) as *mut u64;
        let ptr8 = allocator.alloc(layout_u64) as *mut u64;

        assert!(!ptr5.is_null());
        assert!(!ptr6.is_null());
        assert!(!ptr7.is_null());
        assert!(!ptr8.is_null());

        println!("Heap allocated with 4 u64 numbers");
        print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
        println!();

        *ptr5 = u64::MAX;
        *ptr6 = u64::MAX;
        *ptr7 = u64::MAX;
        *ptr8 = u64::MAX;

        println!("u64 numbers are set to u64::max");
        print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
        println!();

        assert_eq!(*ptr8, u64::MAX);
        assert_eq!(*ptr7, u64::MAX);
        assert_eq!(*ptr6, u64::MAX);
        assert_eq!(*ptr5, u64::MAX);

        allocator.dealloc(ptr8 as *mut u8, layout_u64);
        allocator.dealloc(ptr7 as *mut u8, layout_u64);
        allocator.dealloc(ptr6 as *mut u8, layout_u64);
        allocator.dealloc(ptr5 as *mut u8, layout_u64);

        println!("u64 numbers are deallocated");
        print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
        println!();
    }
}

#[test]
fn linked_list_boundary_conditions() {
    const HEAP_SIZE: usize = 64;
    static mut HEAP_MEM: LinkedListHeap<HEAP_SIZE> = LinkedListHeap::new();

    let allocator = Locked::new(LinkedListAlloc::new());

    unsafe {
        allocator.lock().init(&raw mut HEAP_MEM);

        let layout = Layout::from_size_align(10, 1).unwrap();
        let mut ptrs = Vec::new();

        loop {
            let ptr = allocator.alloc(layout);
            if ptr.is_null() {
                break;
            }
            ptrs.push(ptr);
        }

        assert!(!ptrs.is_empty());

        let ptr = allocator.alloc(layout);
        assert!(ptr.is_null());
    }
}

#[test]
fn linked_list_mixed_allocations() {
    const HEAP_SIZE: usize = 64;
    static mut HEAP_MEM: LinkedListHeap<HEAP_SIZE> = LinkedListHeap::new();

    let allocator = Locked::new(LinkedListAlloc::new());

    unsafe {
        allocator.lock().init(&raw mut HEAP_MEM);

        let layout_u64: Layout = Layout::new::<u64>();
        let layout_array_u8 = Layout::new::<[u8; 3]>();
        let layout_u16 = Layout::new::<u16>();
        let layout_u32 = Layout::new::<u32>();

        let ptr1_u64 = allocator.alloc(layout_u64) as *mut u64;
        let ptr_array_u8 = allocator.alloc(layout_array_u8) as *mut [u8; 3];
        let ptr1_u16 = allocator.alloc(layout_u16) as *mut u16;
        let ptr2_u64 = allocator.alloc(layout_u64) as *mut u64;

        assert!(!ptr1_u64.is_null());
        assert!(!ptr_array_u8.is_null());
        assert!(!ptr1_u16.is_null());
        assert!(!ptr2_u64.is_null());

        *ptr1_u64 = u64::MAX;
        *ptr_array_u8 = [u8::MAX, u8::MAX, u8::MAX];
        *ptr1_u16 = u16::MAX;
        *ptr2_u64 = u64::MAX;

        print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);

        allocator.dealloc(ptr2_u64 as *mut u8, layout_u64);
        allocator.dealloc(ptr_array_u8 as *mut u8, layout_array_u8);

        print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);

        let ptr_u32 = allocator.alloc(layout_u32) as *mut u32;
        *ptr_u32 = u32::MAX;

        assert!(!ptr_u32.is_null());

        print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
    }
}

#[test]
fn buddy_alloc() {
    #[repr(align(8))]
    pub struct Heap<const HEAP_SIZE: usize>(pub [MaybeUninit<u8>; HEAP_SIZE]);

    const HEAP_SIZE: usize = 512;
    static mut HEAP_MEM: Heap<HEAP_SIZE> = Heap([MaybeUninit::uninit(); HEAP_SIZE]);

    let allocator = Locked::new(BuddyAlloc::new());

    unsafe {
        allocator.lock().init(&raw mut HEAP_MEM as usize, HEAP_SIZE);

        println!("{:?}", *allocator.lock());
        print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
        println!();

        allocator.lock().split_area(4).unwrap();

        println!("{:?}", *allocator.lock());
        print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
        println!();

        let t = allocator.lock().list_areas[4]
            .head
            .unwrap()
            .as_ref()
            .start_addr();

        let te = allocator.lock().list_areas[4].head.unwrap().as_ref().next;

        println!("{te:x?}");

        allocator.lock().combine_free_buddies(t, 4);

        println!("{:?}", *allocator.lock());
        print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
        println!();

        // let layout_u8_16 = Layout::new::<[u8; 16]>();

        // let ptr1_u8_16 = allocator.alloc(layout_u8_16) as *mut [u8; 16];
        // *ptr1_u8_16 = [0xFF_u8; 16];

        // println!("{:?}", *allocator.lock());
        // print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
        // println!();

        // allocator.dealloc(ptr1_u8_16 as *mut u8, layout_u8_16);

        // println!("{:?}", *allocator.lock());
        // print_heap_dump(&raw mut HEAP_MEM.0 as *mut u8, HEAP_SIZE);
        // println!();
    }
}
