// pub unsafe fn print_mem(heap: *const u8, len: usize) {
//     unsafe {
//         for i in 0..len {
//             if i % 16 == 0 {
//                 print!("\n{:08x}: ", i);
//             }
//             print!("{:02x} ", *heap.add(i));
//         }
//         println!();
//     }
// }
