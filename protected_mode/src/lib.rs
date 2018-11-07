#![feature(global_asm)]
#![feature(lang_items)]

#![no_std]

use core::panic::PanicInfo;

global_asm!(include_str!("e820.s"));

pub mod stages;

extern "C" {
    pub fn first_stage();
}

#[no_mangle]
pub extern "C" fn hello_world() {
    unsafe { first_stage() }
}

#[panic_handler]
#[no_mangle]
pub extern "C" fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[lang = "eh_personality"]
#[no_mangle]
pub extern "C" fn eh_personality() {
    loop {}
}
