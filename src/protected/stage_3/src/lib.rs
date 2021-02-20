#![no_std]
#![feature(abi_x86_interrupt, asm, global_asm)]

use shared::println;
use shared::instructions;
use shared::linker_symbol;

mod interrupts;
mod panic;
mod ivt;
mod v8086;

use v8086::{Monitor, Stack};

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

    let stack = Stack::new(linker_symbol!(_stack_start), 0x2B);
    let monitor = Monitor::new(stack);
    let function_address = linker_symbol!(v8086_test);

    println!("Entering V8086");

    unsafe {
        //enter_v8086();
        monitor.start(function_address);
    }

    println!("User mode returned");

    loop {};
}