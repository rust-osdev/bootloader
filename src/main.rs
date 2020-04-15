#![no_std]
#![no_main]
#![feature(abi_efiapi)]

use core::panic::PanicInfo;
use uefi::prelude::{entry, Boot, Handle, Status, SystemTable};

#[entry]
fn efi_main(image: Handle, st: SystemTable<Boot>) -> Status {
    Status::SUCCESS
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
