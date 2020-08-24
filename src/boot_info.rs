use crate::memory_map::MemoryRegion;

#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    pub memory_regions: &'static mut [MemoryRegion],
    pub framebuffer: FrameBufferInfo,
}

#[derive(Debug)]
#[repr(C)]
pub struct FrameBufferInfo {
    pub start_addr: u64,
    pub len: usize,
}

extern "C" fn _assert_ffi(_boot_info: &'static mut BootInfo) {}
