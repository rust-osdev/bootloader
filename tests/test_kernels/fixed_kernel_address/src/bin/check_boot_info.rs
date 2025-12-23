#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader_api::{BootInfo, entry_point};
use test_kernel_fixed_kernel_address::{BOOTLOADER_CONFIG, KERNEL_ADDR, QemuExitCode, exit_qemu};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    assert_eq!(boot_info.kernel_image_offset, KERNEL_ADDR);

    exit_qemu(QemuExitCode::Success);
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    let _ = writeln!(test_kernel_fixed_kernel_address::serial(), "PANIC: {info}");
    exit_qemu(QemuExitCode::Failed);
}
