#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader_api::{
    config::Mapping, entry_point, info::MemoryRegionKind, BootInfo, BootloaderConfig,
};
use core::{fmt::Write, ptr::slice_from_raw_parts};
use test_kernel_ramdisk::{exit_qemu, serial, QemuExitCode, RAMDISK_CONTENTS};
use x86_64::{
    structures::paging::{OffsetPageTable, PageTable, PageTableFlags, Translate},
    VirtAddr,
};

pub const BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::FixedAddress(0x0000_6000_0000_0000));
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    writeln!(serial(), "Boot info: {boot_info:?}").unwrap();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    let level_4_table = unsafe { active_level_4_table(phys_mem_offset) };
    let page_table = unsafe { OffsetPageTable::new(level_4_table, phys_mem_offset) };

    let ramdisk_start_addr = VirtAddr::new(boot_info.ramdisk_addr.into_option().unwrap());
    assert_eq!(boot_info.ramdisk_len as usize, RAMDISK_CONTENTS.len());
    let ramdisk_end_addr = ramdisk_start_addr + boot_info.ramdisk_len;

    let mut next_addr = ramdisk_start_addr;
    while next_addr < ramdisk_end_addr {
        let phys_addr = match page_table.translate(next_addr) {
            x86_64::structures::paging::mapper::TranslateResult::Mapped {
                frame,
                offset: _,
                flags,
            } => {
                assert!(flags.contains(PageTableFlags::PRESENT));
                assert!(flags.contains(PageTableFlags::WRITABLE));

                next_addr += frame.size();

                frame.start_address()
            }
            other => panic!("invalid result: {other:?}"),
        };
        let region = boot_info
            .memory_regions
            .iter()
            .find(|r| r.start <= phys_addr.as_u64() && r.end > phys_addr.as_u64())
            .unwrap();
        assert_eq!(region.kind, MemoryRegionKind::Bootloader);
    }

    let actual_ramdisk = unsafe {
        &*slice_from_raw_parts(
            boot_info.ramdisk_addr.into_option().unwrap() as *const u8,
            boot_info.ramdisk_len as usize,
        )
    };
    writeln!(serial(), "Actual contents: {actual_ramdisk:?}").unwrap();
    assert_eq!(RAMDISK_CONTENTS, actual_ramdisk);

    exit_qemu(QemuExitCode::Success);
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let _ = writeln!(test_kernel_ramdisk::serial(), "PANIC: {info}");
    exit_qemu(QemuExitCode::Failed);
}

pub unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr // unsafe
}
