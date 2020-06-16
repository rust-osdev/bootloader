#![no_std]

use shared::println;

mod panic;

#[no_mangle]
pub extern "C" fn third_stage() -> ! {
	println!("[Bootloader] [32] Stage 3");

	unsafe {
	    let ptr = 0x110000 as *mut u32;
	    *ptr = 0xdeadbeef;
    }

    println!("[Bootloader] [32] > 1MB");

	loop {}
}