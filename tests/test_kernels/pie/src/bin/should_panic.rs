#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use test_kernel_pie::{exit_qemu, QemuExitCode};

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    panic!();
}

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    exit_qemu(QemuExitCode::Success);
}
