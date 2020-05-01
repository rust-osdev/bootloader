#![no_std]
#![no_main]

mod panic;

extern crate rlibc;

#[no_mangle]
fn bootloader_no_optimize() {}
