#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader::{boot_info::PixelFormat, entry_point, BootInfo};
use core::panic::PanicInfo;
use test_kernel_default_settings::{exit_qemu, QemuExitCode};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // check memory regions
    assert!(boot_info.memory_regions.len() > 4);

    // check framebuffer
    let framebuffer = boot_info.framebuffer.as_ref().unwrap();
    assert_eq!(framebuffer.info().byte_len, framebuffer.buffer().len());
    if ![640, 1024].contains(&framebuffer.info().horizontal_resolution) {
        panic!(
            "unexpected horizontal_resolution `{}`",
            framebuffer.info().horizontal_resolution
        );
    }
    if ![480, 768].contains(&framebuffer.info().vertical_resolution) {
        panic!(
            "unexpected vertical_resolution `{}`",
            framebuffer.info().vertical_resolution
        );
    }
    if ![3, 4].contains(&framebuffer.info().bytes_per_pixel) {
        panic!(
            "unexpected bytes_per_pixel `{}`",
            framebuffer.info().bytes_per_pixel
        );
    }
    if ![640, 1024].contains(&framebuffer.info().stride) {
        panic!("unexpected stride `{}`", framebuffer.info().stride);
    }
    assert_eq!(framebuffer.info().pixel_format, PixelFormat::BGR);
    assert_eq!(
        framebuffer.buffer().len(),
        framebuffer.info().stride
            * framebuffer.info().vertical_resolution
            * framebuffer.info().bytes_per_pixel
    );

    // check defaults for optional features
    assert_eq!(boot_info.physical_memory_offset.into_option(), None);
    assert_eq!(boot_info.recursive_index.into_option(), None);

    // check rsdp_addr
    let rsdp = boot_info.rsdp_addr.into_option().unwrap();
    assert!(rsdp > 0x000E0000);
    assert!(rsdp < 0x000FFFFF);

    // the test kernel has no TLS template
    assert_eq!(boot_info.tls_template.into_option(), None);

    exit_qemu(QemuExitCode::Success);
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use core::fmt::Write;

    let _ = writeln!(test_kernel_default_settings::serial(), "PANIC: {}", info);
    exit_qemu(QemuExitCode::Failed);
}
