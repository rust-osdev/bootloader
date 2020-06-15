#![no_std]
#![feature(llvm_asm)]

use shared::println;

mod panic;

#[no_mangle]
pub extern "C" fn third_stage() -> ! {
	println!("[Bootloader] [32] Stage 3");

	loop {}
}