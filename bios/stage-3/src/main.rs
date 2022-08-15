#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

mod paging;

#[no_mangle]
#[link_section = ".start"]
pub extern "C" fn _start(stage_4_addr: u32, kernel_addr: u32) {
    // set up identity mapping, enable paging, and switch CPU into long
    // mode (32-bit compatibility mode)
    paging::init();

    // TODO: Set up long mode with identity-mapping, then jump to 4th stage (passing
    // kernel, memory map, and vesa info as arguments)

    let vga = 0xb8000 as *mut u16;

    for i in 0..(80 * 25) {
        unsafe { vga.wrapping_add(i).write_volatile(0x0f01) };
    }

    loop {}
}

#[panic_handler]
pub fn panic(info: &core::panic::PanicInfo) -> ! {
    loop {}
}
