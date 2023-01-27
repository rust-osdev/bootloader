#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader_api::{entry_point, BootInfo};
use core::{fmt::Write, ptr::slice_from_raw_parts};
use test_kernel_ramdisk::{exit_qemu, serial, QemuExitCode, RAMDISK_CONTENTS};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    writeln!(serial(), "Boot info: {boot_info:?}").unwrap();
    assert!(boot_info.ramdisk_addr.into_option().is_some());
    assert_eq!(boot_info.ramdisk_len as usize, RAMDISK_CONTENTS.len());
    let actual_ramdisk = unsafe {
        &*slice_from_raw_parts(
            boot_info.ramdisk_addr.into_option().unwrap() as *const u8,
            boot_info.ramdisk_len as usize,
        )
    };
    writeln!(serial(), "Actual contents: {actual_ramdisk:?}").unwrap();
    assert_eq!(RAMDISK_CONTENTS, actual_ramdisk);

    exit_qemu(QemuExitCode::Success);
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let _ = writeln!(test_kernel_ramdisk::serial(), "PANIC: {info}");
    exit_qemu(QemuExitCode::Failed);
}
