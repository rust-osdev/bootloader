#![no_std]
#![feature(abi_x86_interrupt, asm, global_asm)]

use shared::println;
use shared::instructions;
use shared::linker_symbol;

mod interrupts;
mod panic;

global_asm!(include_str!("iret.s"));

extern "C" {
    fn v8086_test();
    fn iret_asm_test();
}

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

    unsafe {
        let eflags = instructions::read_eflags() ;//| (1 << 17);
        let fn_addr = &iret_test as *const _ as u32;

        println!("fn @ {}", fn_addr);

        iret_asm_test();
    }

    println!("User mode returned");

    loop {};
}

#[no_mangle]
pub extern "C" fn iret_test() {
    println!("User mode");
}