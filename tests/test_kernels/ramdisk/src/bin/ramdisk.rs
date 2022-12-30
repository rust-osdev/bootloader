#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader_api::{entry_point, BootInfo};
use test_kernel_ramdisk::{exit_qemu, QemuExitCode, RAMDISK_CONTENTS};



entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {   
    assert!(boot_info.ramdisk_addr.into_option().is_some());
    assert_eq!(boot_info.ramdisk_len as usize, RAMDISK_CONTENTS.len());

    let ramdisk = boot_info.ramdisk_addr.into_option().unwrap() as *const u8;
    compare_ramdisk_contents(ramdisk);    


    exit_qemu(QemuExitCode::Success);
}

fn compare_ramdisk_contents(ramdisk: *const u8) {
    let expected = RAMDISK_CONTENTS;
    for i in 0..expected.len() {
        unsafe { assert_eq!(expected[i], *ramdisk.offset(i as isize)); }
    }
}
/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    let _ = writeln!(test_kernel_ramdisk::serial(), "PANIC: {}", info);
    exit_qemu(QemuExitCode::Failed);
}
