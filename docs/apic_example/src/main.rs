#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

mod frame_allocator;
mod apic;
mod idt;
mod gdt;

extern crate alloc;

use bootloader_api::{entry_point, BootInfo};
use x86_64::structures::paging::OffsetPageTable;
use x86_64::VirtAddr;
use bootloader_api::config::Mapping;
use crate::frame_allocator::BootInfoFrameAllocator;

pub const CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &CONFIG);

pub fn kernel_main(boot_info: &'static mut BootInfo) {
    let physical_memory_offset = VirtAddr::new(
        boot_info
            .physical_memory_offset
            .take()
            .expect("Failed to find physical memory offset"),
    );
    let mut mapper: OffsetPageTable<'static> = frame_allocator::init(physical_memory_offset);
    let mut frame_allocator = BootInfoFrameAllocator::new(&boot_info.memory_regions);

    let rsdp: Option<u64> = boot_info.rsdp_addr.take();

    unsafe {
        apic::init(rsdp.expect("Failed to get RSDP address") as usize, physical_memory_offset, &mut mapper, &mut frame_allocator);
    }
}