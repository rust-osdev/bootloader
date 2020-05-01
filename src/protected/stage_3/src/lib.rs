#![no_std]
#![feature(llvm_asm)]

use shared::println;

mod panic;

#[no_mangle]
pub extern "C" fn third_stage() {
	unsafe {
		llvm_asm!("mov bx, 0x0
		           mov ds, bx
		           mov es, bx" ::: "bx" : "intel", "volatile");
	}

	println!("Stage 3");
}