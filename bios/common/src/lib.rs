#![no_std]

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Addresses {
    pub stage_4: Region,
    pub kernel: Region,
    pub memory_map: Region,
    pub framebuffer: Region,
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Region {
    pub start: u64,
    pub len: u64,
}
