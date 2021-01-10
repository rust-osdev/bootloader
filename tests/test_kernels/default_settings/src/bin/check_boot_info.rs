#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader::{boot_info::PixelFormat, entry_point, BootInfo};
use core::panic::PanicInfo;
use kernel::{exit_qemu, QemuExitCode};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // check memory regions
    assert!(boot_info.memory_regions.len() > 4);

    // check framebuffer
    let framebuffer = boot_info.framebuffer.as_ref().unwrap();
    assert_eq!(framebuffer.info().byte_len, framebuffer.buffer().len());
    assert_eq!(framebuffer.info().horizontal_resolution, 1024);
    assert_eq!(framebuffer.info().vertical_resolution, 768);
    assert_eq!(framebuffer.info().bytes_per_pixel, 3);
    assert_eq!(framebuffer.info().stride, 1024);
    assert_eq!(framebuffer.info().pixel_format, PixelFormat::RGB);
    assert_eq!(framebuffer.buffer().len(), 1024 * 768 * 3);

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

    let _ = writeln!(kernel::serial(), "PANIC: {}", info);
    exit_qemu(QemuExitCode::Failed);
}
