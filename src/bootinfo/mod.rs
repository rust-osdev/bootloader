#![deny(improper_ctypes)]

pub use self::memory_map::*;

mod memory_map;

#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    pub memory_map: MemoryMap,
    /// The virtual address of the recursively mapped level 4 page table.
    #[cfg(feature = "recursive_page_table")]
    pub recursive_page_table_addr: u64,
    /// The offset into the virtual address space where the physical memory is mapped.
    ///
    /// Physical addresses can be converted to virtual addresses by adding this offset to them.
    ///
    /// The mapping of the physical memory allows to access arbitrary physical frames. Accessing
    /// frames that are also mapped at other virtual addresses can easily break memory safety and
    /// cause undefined behavior. Only frames reported as `USABLE` by the memory map in the `BootInfo`
    /// can be safely accessed.
    #[cfg(feature = "map_physical_memory")]
    pub physical_memory_offset: u64,
    _non_exhaustive: u8, // `()` is not FFI safe
}

impl BootInfo {
    #[allow(unused_variables)]
    pub fn new(memory_map: MemoryMap, recursive_page_table_addr: u64, physical_memory_offset: u64) -> Self {
        BootInfo {
            memory_map,
            #[cfg(feature = "recursive_page_table")]
            recursive_page_table_addr,
            #[cfg(feature = "map_physical_memory")]
            physical_memory_offset,
            _non_exhaustive: 0,
        }
    }
}

extern "C" {
    fn _improper_ctypes_check(_boot_info: BootInfo);
}
