#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader_api::{entry_point, BootInfo};
use test_kernel_fixed_kernel_address::{exit_qemu, QemuExitCode, BOOTLOADER_CONFIG, KERNEL_ADDR};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // verify that kernel is loaded at the specified base address.
    let rip = x86_64::registers::read_rip().as_u64();
    let kernel_start = KERNEL_ADDR;
    let kernel_end = kernel_start + boot_info.kernel_len;
    let kernel_range = kernel_start..kernel_end;

    assert!(kernel_range.contains(&rip));

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
