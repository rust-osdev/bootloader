#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader_api::{entry_point, BootInfo};
use core::sync::atomic::{AtomicU64, Ordering};
use test_kernel_pie::{exit_qemu, QemuExitCode};

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    // Initialize with a value that is unlikely to be anywhere in memory.
    // If we can later read out this exact value, we can be sure that we actually
    // read from this global variable and not some other location in memory.
    static FOO: AtomicU64 = AtomicU64::new(0xdeadbeef);

    // Make sure that relocations are actually applied by referencing a `FOO`
    // in `FOO_REF`. `FOO`'s address will be calculated and put into `FOO_REF`
    // at load time using a relocation.
    static FOO_REF: &AtomicU64 = &FOO;

    // Verify that the memory address pointed to by `FOO_REF` contains our value.
    let val = FOO_REF.load(Ordering::Relaxed);
    assert_eq!(val, 0xdeadbeef);

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
