#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn rust_main() -> u32 {
    54321
}

#[panic_handler]
pub fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop{}
}
