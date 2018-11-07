#![no_std]
#![no_main]

use core::panic::PanicInfo;

extern "C" {
    fn hello_world();
}

#[no_mangle]
pub extern "C" fn entry_point() {
    unsafe {hello_world();}
}

#[panic_handler]
extern "C" fn panic(info: &PanicInfo) -> ! {
    loop {}
}
