#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader_api::{
    config::Mapping, entry_point, info::MemoryRegionKind, BootInfo, BootloaderConfig,
};
use test_kernel_write_usable_memory::{exit_qemu, QemuExitCode};

pub const BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::FixedAddress(0x0000_4000_0000_0000));
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let phys_mem_offset = boot_info.physical_memory_offset.into_option().unwrap();

    for region in boot_info.memory_regions.iter() {
        if region.kind == MemoryRegionKind::Usable {
            // ensure region is actually writable
            let addr = phys_mem_offset + region.start;
            let size = region.end - region.start;
            unsafe {
                core::ptr::write_bytes(addr as *mut u8, 0xff, size as usize);
            }
        }
    }

    exit_qemu(QemuExitCode::Success);
}

/// This function is called on panic.
#[panic_handler]
#[cfg(not(test))]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    let _ = writeln!(test_kernel_write_usable_memory::serial(), "PANIC: {}", info);
    exit_qemu(QemuExitCode::Failed);
}
