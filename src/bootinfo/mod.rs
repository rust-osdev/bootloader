#![deny(improper_ctypes)]

pub use self::memory_map::*;
use core::ops::Deref;
use core::slice;

mod memory_map;

#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    pub p4_table_addr: u64,
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
    pub fn new(p4_table_addr: u64, memory_map: MemoryMap, package: &'static [u8]) -> Self {
        BootInfo {
            p4_table_addr,
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

use x86_64::{
    structures::paging::{PhysFrame, PhysFrameRange},
    PhysAddr,
};

impl From<FrameRange> for PhysFrameRange {
    fn from(range: FrameRange) -> Self {
        PhysFrameRange {
            start: PhysFrame::from_start_address(PhysAddr::new(range.start_addr())).unwrap(),
            end: PhysFrame::from_start_address(PhysAddr::new(range.end_addr())).unwrap(),
        }
    }
}

impl From<PhysFrameRange> for FrameRange {
    fn from(range: PhysFrameRange) -> Self {
        FrameRange::new(
            range.start.start_address().as_u64(),
            range.end.start_address().as_u64(),
        )
    }
}
