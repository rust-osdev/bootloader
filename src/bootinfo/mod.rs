#![deny(improper_ctypes)]

pub use self::memory_map::*;
use core::ops::Deref;
use core::slice;

mod memory_map;

#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    pub memory_map: MemoryMap,
}

impl BootInfo {
    pub fn new(memory_map: MemoryMap) -> Self {
        BootInfo {
            memory_map,
        }
    }
}

extern "C" {
    fn _improper_ctypes_check(_boot_info: BootInfo);
}
