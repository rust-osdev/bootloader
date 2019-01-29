#![deny(improper_ctypes)]

pub use self::memory_map::*;

mod memory_map;

#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    pub memory_map: MemoryMap,
    _non_exhaustive: u8, // `()` is not FFI safe
}

impl BootInfo {
    pub fn new(memory_map: MemoryMap) -> Self {
        BootInfo {
            memory_map,
            _non_exhaustive: 0,
        }
    }
}

extern "C" {
    fn _improper_ctypes_check(_boot_info: BootInfo);
}
