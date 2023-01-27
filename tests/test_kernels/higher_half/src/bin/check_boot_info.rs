#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader_api::{entry_point, info::PixelFormat, BootInfo};
use test_kernel_higher_half::{exit_qemu, QemuExitCode, BOOTLOADER_CONFIG};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // check memory regions
    assert!(boot_info.memory_regions.len() > 4);

    // check framebuffer
    let framebuffer = boot_info.framebuffer.as_ref().unwrap();
    assert_eq!(framebuffer.info().byte_len, framebuffer.buffer().len());
    if ![3, 4].contains(&framebuffer.info().bytes_per_pixel) {
        panic!(
            "unexpected bytes_per_pixel `{}`",
            framebuffer.info().bytes_per_pixel
        );
    }
    assert_eq!(framebuffer.info().pixel_format, PixelFormat::Bgr);
    assert_eq!(
        framebuffer.buffer().len(),
        framebuffer.info().stride * framebuffer.info().height * framebuffer.info().bytes_per_pixel
    );

    // check defaults for optional features
    assert_eq!(boot_info.physical_memory_offset.into_option(), None);
    assert_eq!(boot_info.recursive_index.into_option(), None);

    // check rsdp_addr
    let rsdp = boot_info.rsdp_addr.into_option().unwrap();
    assert!(rsdp > 0x000E0000);

    // the test kernel has no TLS template
    assert_eq!(boot_info.tls_template.into_option(), None);

    exit_qemu(QemuExitCode::Success);
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    let _ = writeln!(test_kernel_higher_half::serial(), "PANIC: {info}");
    exit_qemu(QemuExitCode::Failed);
}
