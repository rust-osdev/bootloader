use core::slice;

use x86_64::VirtAddr;
use usize_conversions::usize_from;
use os_bootinfo::{BootInfo, MemoryRegion, E820MemoryRegion};

pub(crate) fn create_from(memory_map_addr: VirtAddr, entry_count: u64) -> BootInfo {
    let memory_map_start_ptr = usize_from(memory_map_addr.as_u64()) as *const E820MemoryRegion;
    let e820_memory_map = unsafe {
        slice::from_raw_parts(memory_map_start_ptr, usize_from(entry_count))
    };

    let mut boot_info = BootInfo::new();
    for region in e820_memory_map {
        boot_info.memory_map.push(MemoryRegion::from(*region));
    }

    boot_info
}
