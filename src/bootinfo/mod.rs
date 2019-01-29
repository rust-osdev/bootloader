#![deny(improper_ctypes)]

pub use self::memory_map::*;
use core::ops::Deref;
use core::slice;

mod memory_map;

#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    pub memory_map: MemoryMap,
    pub package: Package,
}

#[derive(Debug)]
#[repr(C)]
pub struct Package {
    ptr: *const u8,
    len: u64,
}

impl Deref for Package {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr, self.len as usize) }
    }
}

impl BootInfo {
    pub fn new(memory_map: MemoryMap, package: &'static [u8]) -> Self {
        BootInfo {
            memory_map,
            package: Package {
                ptr: package.as_ptr(),
                len: package.len() as u64,
            },
        }
    }
}

extern "C" {
    fn _improper_ctypes_check(_boot_info: BootInfo);
}
