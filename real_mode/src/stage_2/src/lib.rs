#![no_std]

use shared::console::println;

#[no_mangle]
pub fn second_stage() -> u16 {
	println(b"Stage 2");
    return 12345;
}