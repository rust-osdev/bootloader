#![no_std]

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct BiosInfo {
    pub stage_4: Region,
    pub kernel: Region,
    pub memory_map: Region,
    pub framebuffer: FramebufferInfo,
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct FramebufferInfo {
    pub region: Region,
    pub width: u16,
    pub height: u16,
    pub bytes_per_pixel: u8,
    pub stride: u16,
    pub pixel_format: PixelFormat,
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Region {
    pub start: u64,
    pub len: u64,
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub enum PixelFormat {
    Rgb,
    Bgr,
    Unknown {
        red_position: u8,
        green_position: u8,
        blue_position: u8,
    },
}
