#![feature(lang_items)]
#![feature(global_asm)]
#![no_std]
#![no_main]

extern crate rlibc;
extern crate xmas_elf;

use core::slice;

global_asm!(include_str!("boot.s"));
global_asm!(include_str!("second_stage.s"));
global_asm!(include_str!("kernel.s"));

#[no_mangle]
pub extern "C" fn load_elf(kernel_start: *const u8, kernel_size: usize) -> ! {
    let kernel = unsafe { slice::from_raw_parts(kernel_start, kernel_size) };
    let elf_file = xmas_elf::ElfFile::new(kernel).unwrap();
    xmas_elf::header::sanity_check(&elf_file).unwrap();
    loop {}
}

#[lang = "panic_fmt"]
#[no_mangle]
pub extern fn rust_begin_panic(_msg: core::fmt::Arguments,
                               _file: &'static str,
                               _line: u32,
                               _column: u32) -> ! {
    const VGA_BUFFER: *mut u8 = 0xb8000 as *mut _;

    unsafe {
        let vga_buffer = slice::from_raw_parts_mut(VGA_BUFFER, 25 * 80 *2);
        vga_buffer[0] = b'E'; vga_buffer[1] = 0x4f;
        vga_buffer[2] = b'R'; vga_buffer[3] = 0x4f;
        vga_buffer[4] = b'R'; vga_buffer[5] = 0x4f;
    }

    loop {}
}

#[lang = "eh_personality"]
#[no_mangle]
pub extern fn eh_personality() { loop {} }

#[no_mangle]
pub extern fn _Unwind_Resume() { loop {} }
