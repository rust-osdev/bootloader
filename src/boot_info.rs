use core::{mem, slice};

use x86_64::VirtAddr;
use usize_conversions::{usize_from, FromUsize};
use os_bootinfo::{BootInfo, E820MemoryRegion};

pub(crate) fn create_from(memory_map_addr: VirtAddr, entry_count: u64)
    -> &'static mut BootInfo
{
    let memory_map_start_ptr = usize_from(memory_map_addr.as_u64()) as *mut E820MemoryRegion;
    let memory_map = unsafe {
        slice::from_raw_parts_mut(memory_map_start_ptr, usize_from(entry_count))
    };

    let memory_map_end = memory_map_addr +
        entry_count * u64::from_usize(mem::size_of::<E820MemoryRegion>());
    let boot_info_ptr = memory_map_end.as_u64() as *mut BootInfo;
    unsafe {
        boot_info_ptr.write(BootInfo { memory_map });
        &mut *boot_info_ptr
    }
}
