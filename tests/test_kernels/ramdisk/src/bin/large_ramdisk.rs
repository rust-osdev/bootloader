#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader_api::{BootInfo, entry_point};
use core::fmt::Write;
use test_kernel_ramdisk::{QemuExitCode, exit_qemu, serial};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    writeln!(serial(), "Boot info: {boot_info:?}").unwrap();
    assert!(boot_info.ramdisk_addr.into_option().is_some());
    writeln!(serial(), "RAM disk size: {}", boot_info.ramdisk_len).unwrap();

    exit_qemu(QemuExitCode::Success);
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let _ = writeln!(test_kernel_ramdisk::serial(), "PANIC: {info}");
    exit_qemu(QemuExitCode::Failed);
}
