#![no_std]
#![no_main]

#[no_mangle]
#[link_section = ".start"]
pub extern "C" fn _start() {
    loop {}
}

#[panic_handler]
pub fn panic(info: &core::panic::PanicInfo) -> ! {
    loop {}
}
