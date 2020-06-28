#![feature(asm, global_asm)]
#![no_std]
#![allow(dead_code)]

mod console;
mod errors;

use self::console::real_mode_println;
use core::panic::PanicInfo;
use shared::{dap, linker_symbol, instructions};

extern "C" {
    fn second_stage() -> !;
}
global_asm!(include_str!("bootstrap.s"));

#[no_mangle]
extern "C" fn rust_start(disk_number: u16) -> ! {
    real_mode_println(b"[Bootloader] [16] Bootsector");

    check_int13h_extensions(disk_number);

    let dap = dap::DiskAddressPacket::new(
        linker_symbol!(_rest_of_bootloader_start) as u16,
        (linker_symbol!(_rest_of_bootloader_start) - linker_symbol!(_bootloader_start)) as u64,
        linker_symbol!(_rest_of_bootloader_end) - linker_symbol!(_rest_of_bootloader_start),
    );

    unsafe {
        dap.perform_load(disk_number);
        second_stage();
    };
}

fn check_int13h_extensions(disk_number: u16) {
    unsafe {
        asm!("
            int 0x13
            jc no_int13h_extensions",

            in("ax") 0x41, in("bx") 0x55aa, in("dx") disk_number,
            options(nostack)
        )
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    real_mode_println(b"[Panic]");

    loop {
        instructions::hlt()
    }
}
