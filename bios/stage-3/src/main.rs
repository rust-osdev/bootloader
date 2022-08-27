#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

use crate::vga_buffer::Writer;
use bootloader_x86_64_bios_common::Addresses;
use core::{arch::asm, fmt::Write as _};

mod paging;
mod vga_buffer;

#[no_mangle]
#[link_section = ".start"]
pub extern "C" fn _start(addresses: &Addresses) {
    // Writer.clear_screen();
    writeln!(Writer, "Third Stage ({addresses:#x?})").unwrap();

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
