#![feature(llvm_asm, global_asm)]
#![no_std]
#![allow(dead_code)]

mod console;
mod errors;

use self::console::real_mode_println;
use core::panic::PanicInfo;
use shared::{dap, linker_symbol, utils};

extern "C" {
    fn second_stage();
}
global_asm!(include_str!("bootstrap.s"));

#[no_mangle]
extern "C" fn rust_start(disk_number: u16) -> ! {
    real_mode_println(b"Stage 1");

    check_int13h_extensions(disk_number);

    let dap = dap::DiskAddressPacket::new(
        linker_symbol!(_rest_of_bootloader_start) as u16,
        (linker_symbol!(_rest_of_bootloader_start) - linker_symbol!(_bootloader_start)) as u64,
        linker_symbol!(_rest_of_bootloader_end) - linker_symbol!(_rest_of_bootloader_start),
    );

    unsafe { dap.perform_load(disk_number) };

    unsafe { second_stage() };

    loop {
        utils::hlt();
    }
}

fn check_int13h_extensions(disk_number: u16) {
    unsafe {
        llvm_asm!("
			int 0x13
    		jc no_int13h_extensions
        " :: "{ah}"(0x41), "{bx}"(0x55aa), "{dl}"(disk_number) :: "intel", "volatile");
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    real_mode_println(b"[Panic]");

    loop {
        utils::hlt()
    }
}
