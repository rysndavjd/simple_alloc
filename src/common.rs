use spin::{Mutex, MutexGuard};

pub struct Locked<A> {
    inner: Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, A> {
        self.inner.lock()
    }
}

pub fn align_up(addr: usize, align: usize) -> usize {
    let offset = (addr as *const u8).align_offset(align);
    addr + offset
}

pub fn print_heap_dump(heap: *const u8, len: usize) {
    unsafe {
        for i in 0..len {
            if i % 16 == 0 {
                print!("\n{:08x}: ", i);
            }
            print!("{:02x} ", *heap.add(i));
        }
        println!();
    }
}
