#![feature(global_asm)]
#![feature(lang_items)]

#![no_std]

global_asm!(include_str!("e820.s"));

pub mod stages;
mod printer;

use core::panic::PanicInfo;

#[panic_handler]
extern "C" fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[lang = "eh_personality"]
#[no_mangle]
extern "C" fn eh_personality() {
    loop {}
}
