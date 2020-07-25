#![no_std]
#![feature(abi_x86_interrupt, asm)]

use shared::println;
use shared::instructions;

mod interrupts;
mod panic;

#[no_mangle]
pub extern "C" fn third_stage() -> ! {
	println!("[Bootloader] [32] Stage 3");

	unsafe {
	    let ptr = 0x110000 as *mut u32;
	    *ptr = 0xdeadbeef;
    }

    println!("[Bootloader] [32] > 1MB");

    // Load the TSS
    unsafe {
        instructions::ltr(0x2B)
    };

    println!("[Bootloader] [32] Loaded TSS");

    interrupts::init_idt();

    println!("[Bootloader] [32] Loaded IDT");

    loop {};
}