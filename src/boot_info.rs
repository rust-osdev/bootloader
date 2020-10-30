use crate::memory_map::MemoryRegion;
use core::slice;

#[derive(Debug)]
pub struct BootInfo {
    pub memory_regions: &'static mut [MemoryRegion],
    pub framebuffer: Option<FrameBuffer>,
    pub physical_memory_offset: Option<u64>,
    pub recursive_index: Option<u16>,
    pub rsdp_addr: Option<u64>,
    pub(crate) _non_exhaustive: (),
}

#[derive(Debug)]
pub struct FrameBuffer {
    pub(crate) buffer_start: u64,
    pub(crate) buffer_byte_len: usize,
    pub(crate) info: FrameBufferInfo,
}

impl FrameBuffer {
    pub fn buffer(&mut self) -> &mut [u8] {
        unsafe { self.create_buffer() }
    }

    unsafe fn create_buffer<'a>(&self) -> &'a mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.buffer_start as *mut u8, self.buffer_byte_len) }
    }

    pub fn info(&self) -> FrameBufferInfo {
        self.info
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FrameBufferInfo {
    pub byte_len: usize,
    pub horizontal_resolution: usize,
    pub vertical_resolution: usize,
    pub pixel_format: PixelFormat,
    pub bytes_per_pixel: usize,
    pub stride: usize,
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum PixelFormat {
    RGB,
    BGR,
    U8,
}

/// Check that the _pointer_ is FFI-safe.
///
/// Note that the `BootInfo` struct is not FFI-safe, so it needs to be compiled by the same Rust
/// compiler as the kernel in order to be safely accessed.
extern "C" fn _assert_ffi(_boot_info: &'static mut BootInfo) {}
