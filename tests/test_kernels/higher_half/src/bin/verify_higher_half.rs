#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use test_kernel_higher_half::{exit_qemu, QemuExitCode};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // verify that kernel is really running in the higher half of the address space
    // (set in `x86_64-higher_half.json` custom target)
    let rip = x86_64::registers::read_rip().as_u64();
    assert_eq!(rip & 0xffffffffffff0000, 0xffff800000000000);

    // verify that the boot info is located in the higher half of the address space
    assert_eq!(
        (boot_info as *const _ as usize) & 0xffff800000000000,
        0xffff800000000000
    );

    // verify that the stack is located in the higher half of the address space.
    let stack_addr = &rip;
    assert_eq!(
        (stack_addr as *const _ as usize) & 0xffff800000000000,
        0xffff800000000000
    );

    exit_qemu(QemuExitCode::Success);
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use core::fmt::Write;

    let _ = writeln!(test_kernel_higher_half::serial(), "PANIC: {}", info);
    exit_qemu(QemuExitCode::Failed);
}
