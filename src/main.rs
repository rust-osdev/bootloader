#![feature(lang_items)]

#![no_std]
#![no_main]

use core::panic::PanicInfo;

extern "C" {
    fn first_stage();
}

#[no_mangle]
pub extern "C" fn ensure_that_first_stage_is_callable() {
    unsafe {first_stage();}
}

#[panic_handler]
extern "C" fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[lang = "eh_personality"]
#[no_mangle]
pub extern "C" fn eh_personality() {
    loop {}
}
