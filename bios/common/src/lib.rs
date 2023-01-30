#![no_std]

pub mod racy_cell;

#[cfg_attr(feature = "debug", derive(Debug))]
#[repr(C)]
pub struct BiosInfo {
    pub stage_4: Region,
    pub kernel: Region,
    pub ramdisk: Region,
    pub config_file: Region,
    pub last_used_addr: u64,
    pub framebuffer: BiosFramebufferInfo,
    pub memory_map_addr: u32,
    pub memory_map_len: u16,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BiosFramebufferInfo {
    pub region: Region,
    pub width: u16,
    pub height: u16,
    pub bytes_per_pixel: u8,
    pub stride: u16,
    pub pixel_format: PixelFormat,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Region {
    pub start: u64,
    pub len: u64,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Clone, Copy)]
#[repr(C)]
pub enum PixelFormat {
    Rgb,
    Bgr,
    Unknown {
        red_position: u8,
        green_position: u8,
        blue_position: u8,
    },
}

impl PixelFormat {
    pub fn is_unknown(&self) -> bool {
        match self {
            PixelFormat::Rgb | PixelFormat::Bgr => false,
            PixelFormat::Unknown { .. } => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct E820MemoryRegion {
    pub start_addr: u64,
    pub len: u64,
    pub region_type: u32,
    pub acpi_extended_attributes: u32,
}

pub fn hlt() {
    unsafe { core::arch::asm!("hlt") };
}
