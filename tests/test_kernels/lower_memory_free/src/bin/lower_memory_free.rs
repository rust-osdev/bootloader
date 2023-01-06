#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader_api::{entry_point, info::MemoryRegionKind, BootInfo};
use test_kernel_lower_memory_free::{exit_qemu, QemuExitCode};

const LOWER_MEMORY_END_PAGE: u64 = 0x0010_0000;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    use core::fmt::Write;
    use test_kernel_lower_memory_free::serial;

    let mut count = 0;
    for region in boot_info.memory_regions.iter() {
        writeln!(
            serial(),
            "Region: {:016x}-{:016x} - {:?}",
            region.start,
            region.end,
            region.kind
        )
        .unwrap();
        if region.end <= LOWER_MEMORY_END_PAGE && region.kind == MemoryRegionKind::Usable {
            let pages = (region.end - region.start) / 4096;
            count += pages;
        }
    }

    writeln!(serial(), "Free lower memory page count: {}", count).unwrap();
    assert!(count > 0x10); // 0x20 chosen arbirarily, we need _some_ free conventional memory, but not all of it. Some, especially on BIOS, may be reserved for hardware.
    exit_qemu(QemuExitCode::Success);
}

/// This function is called on panic.
#[panic_handler]
#[cfg(not(test))]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    let _ = writeln!(test_kernel_lower_memory_free::serial(), "PANIC: {}", info);
    exit_qemu(QemuExitCode::Failed);
}
