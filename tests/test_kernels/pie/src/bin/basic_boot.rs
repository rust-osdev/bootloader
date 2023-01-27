#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader_api::{entry_point, BootInfo};
use test_kernel_pie::{exit_qemu, QemuExitCode};

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    exit_qemu(QemuExitCode::Success);
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    let _ = writeln!(test_kernel_pie::serial(), "PANIC: {info}");
    exit_qemu(QemuExitCode::Failed);
}
