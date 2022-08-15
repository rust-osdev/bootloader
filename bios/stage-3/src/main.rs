#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

use core::fmt::Write as _;

use crate::vga_buffer::Writer;

mod paging;
mod vga_buffer;

#[no_mangle]
#[link_section = ".start"]
pub extern "C" fn _start(stage_4_addr: u32, kernel_addr: u32) {
    // Writer.clear_screen();
    writeln!(
        Writer,
        "Third Stage (stage_4_addr: {stage_4_addr:#x}, kernel_addr: {kernel_addr:#x})"
    );

    // set up identity mapping, enable paging, and switch CPU into long
    // mode (32-bit compatibility mode)
    paging::init();

    // TODO: Set up long mode with identity-mapping, then jump to 4th stage (passing
    // kernel, memory map, and vesa info as arguments)

    writeln!(Writer, "Paging init done");

    loop {}
}

#[panic_handler]
pub fn panic(info: &core::panic::PanicInfo) -> ! {
    writeln!(Writer, "PANIC: {info}");
    loop {}
}
