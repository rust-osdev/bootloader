#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use test_kernel_map_phys_mem::{exit_qemu, serial, QemuExitCode};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let phys_mem_offset = boot_info.physical_memory_offset.into_option().unwrap();

    let ptr = phys_mem_offset as *const u64;
    let _ = unsafe { *ptr };

    exit_qemu(QemuExitCode::Success);
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use core::fmt::Write;

    let _ = writeln!(serial(), "PANIC: {}", info);
    exit_qemu(QemuExitCode::Failed);
}
