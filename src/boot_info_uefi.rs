use crate::memory_map::MemoryMap;

#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    pub memory_map: &'static mut MemoryMap,
    pub framebuffer: FrameBufferInfo,
}

#[derive(Debug)]
#[repr(C)]
pub struct FrameBufferInfo {
    pub start_addr: u64,
    pub len: usize,
}
