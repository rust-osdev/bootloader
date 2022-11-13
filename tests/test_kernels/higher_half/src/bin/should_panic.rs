#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader_api::{entry_point, BootInfo};
use test_kernel_higher_half::BOOTLOADER_CONFIG;

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    panic!();
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    use test_kernel_higher_half::{exit_qemu, QemuExitCode};

    exit_qemu(QemuExitCode::Success);
}
