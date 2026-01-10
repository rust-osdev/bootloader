#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader_api::{BootInfo, entry_point};
use test_kernel_stack_address::{BOOTLOADER_CONFIG, QemuExitCode, exit_qemu};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {

    let x: i32 = 42;
    let vaddr = &x as *const _ as usize as u64;

    assert_ne!(boot_info.kernel_stack_bottom, 0);
    assert_eq!(boot_info.kernel_stack_len, BOOTLOADER_CONFIG.kernel_stack_size);
    assert!(vaddr >= boot_info.kernel_stack_bottom);
    assert!(vaddr < boot_info.kernel_stack_bottom + boot_info.kernel_stack_len);

    exit_qemu(QemuExitCode::Success);
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    let _ = writeln!(test_kernel_stack_address::serial(), "PANIC: {info}");
    exit_qemu(QemuExitCode::Failed);
}
