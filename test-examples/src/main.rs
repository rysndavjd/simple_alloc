#![no_std]
#![no_main]

use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};
use core::panic::PanicInfo;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    loop {}
}

#[entry]
fn main() -> ! {
    hprintln!("Hello, world!");

    debug::exit(debug::EXIT_SUCCESS);

    loop {}
}