// info taken from https://wiki.osdev.org/VESA_Video_Modes

use bootloader_x86_64_bios_common::PixelFormat;

use crate::{disk::AlignedBuffer, AlignedArrayBuffer};
use core::arch::asm;

#[repr(C, packed)]
#[allow(dead_code)]
struct VbeInfoBlock {
    signature: [u8; 4], // should be "VESA"
    version: u16,       // should be 0x0300 for VBE 3.0
    oem_string_ptr: u32,
    capabilities: u32,
    video_mode_ptr: u32,
    total_memory: u16, // number of 64KB blocks
    oem: [u8; 512 - 0x14],
}

pub struct VesaInfo<'a> {
    /// We must store a reference to the full block instead of only copying the
    /// required information out because the video mode pointer might point inside the
    /// `oem` field.
    ///
    /// See https://www.ctyme.com/intr/rb-0273.htm for details.
    info_block: &'a VbeInfoBlock,
    rest_of_buffer: &'a mut [u8],
}

impl<'a> VesaInfo<'a> {
    pub fn query<const N: usize>(buffer: &'a mut AlignedArrayBuffer<N>) -> Result<Self, u16> {
        assert_eq!(core::mem::size_of::<VbeInfoBlock>(), 512);

        let (slice, rest_of_buffer) = buffer
            .slice_mut()
            .split_at_mut(core::mem::size_of::<VbeInfoBlock>());
        slice.fill(0);
        let block_ptr = slice.as_mut_ptr();
        let ret;
        unsafe {
            asm!("push es", "mov es, {:x}", "int 0x10", "pop es", in(reg)0, inout("ax") 0x4f00u16 => ret, in("di") block_ptr)
        };
        match ret {
            0x4f => {
                let info_block: &VbeInfoBlock = unsafe { &*block_ptr.cast() };
                Ok(VesaInfo {
                    info_block,
                    rest_of_buffer,
                })
            }
            other => Err(other),
        }
    }

    pub fn get_best_mode(
        &mut self,
        max_width: u16,
        max_height: u16,
    ) -> Result<Option<VesaModeInfo>, u16> {
        let mut best: Option<VesaModeInfo> = None;
        for i in 0.. {
            let mode = match self.get_mode(i) {
                Some(mode) => mode,
                None => break,
            };
            let mode_info = VesaModeInfo::query(mode, self.rest_of_buffer).unwrap();

            if mode_info.attributes & 0x90 != 0x90 {
                // not a graphics mode with linear frame buffer support
                continue;
            }

            let supported_modes = [
                4u8, // packed pixel graphics
                6,   // direct color (24-bit color)
            ];
            if !supported_modes.contains(&mode_info.memory_model) {
                // unsupported mode
                continue;
            }

            if mode_info.width > max_width || mode_info.height > max_height {
                continue;
            }

            let replace = match &best {
                None => true,
                Some(best) => {
                    best.pixel_format.is_unknown()
                        || best.width < mode_info.width
                        || (best.width == mode_info.width && best.height < mode_info.height)
                }
            };

            if replace {
                best = Some(mode_info);
            }
        }
        Ok(best)
    }

    fn get_mode(&self, index: usize) -> Option<u16> {
        let (segment, offset) = {
            let raw = self.info_block.video_mode_ptr;
            ((raw >> 16) as u16, raw as u16)
        };
        let video_mode_ptr = ((segment as u32) << 4) + offset as u32;

        let base_ptr = video_mode_ptr as *const u16;
        let ptr = unsafe { base_ptr.add(index) };
        let mode = unsafe { *ptr };
        if mode == 0xffff {
            None
        } else {
            Some(mode)
        }
    }
}

#[derive(Debug)]
pub struct VesaModeInfo {
    mode: u16,
    pub width: u16,
    pub height: u16,
    pub framebuffer_start: u32,
    pub bytes_per_scanline: u16,
    pub bytes_per_pixel: u8,
    pub pixel_format: PixelFormat,

    memory_model: u8,
    attributes: u16,
}

impl VesaModeInfo {
    fn query(mode: u16, buffer: &mut [u8]) -> Result<Self, u16> {
        #[repr(C, align(256))]
        struct VbeModeInfo {
            attributes: u16,
            window_a: u8,
            window_b: u8,
            granularity: u16,
            window_size: u16,
            segment_a: u16,
            segment_b: u16,
            window_function_ptr: u32,
            bytes_per_scanline: u16,
            width: u16,
            height: u16,
            w_char: u8,
            y_char: u8,
            planes: u8,
            bits_per_pixel: u8,
            banks: u8,
            memory_model: u8,
            bank_size: u8,
            image_pages: u8,
            reserved_0: u8,
            red_mask: u8,
            red_position: u8,
            green_mask: u8,
            green_position: u8,
            blue_mask: u8,
            blue_position: u8,
            reserved_mask: u8,
            reserved_position: u8,
            direct_color_attributes: u8,
            framebuffer: u32,
            off_screen_memory_offset: u32,
            off_screen_memory_size: u16,
            reserved: [u8; 206],
        }

        assert_eq!(core::mem::size_of::<VbeModeInfo>(), 256);

        let slice = &mut buffer[..core::mem::size_of::<VbeModeInfo>()];
        slice.fill(0);
        let block_ptr = slice.as_mut_ptr();

        let mut ret: u16;
        let mut target_addr = block_ptr as u32;
        let segment = target_addr >> 4;
        target_addr -= segment << 4;
        unsafe {
            asm!(
                "push es", "mov es, {:x}", "int 0x10", "pop es",
                in(reg) segment as u16,
                inout("ax") 0x4f01u16 => ret,
                in("cx") mode,
                in("di") target_addr as u16
            )
        };
        match ret {
            0x4f => {
                let block: &VbeModeInfo = unsafe { &*block_ptr.cast() };
                Ok(VesaModeInfo {
                    mode,
                    width: block.width,
                    height: block.height,
                    framebuffer_start: block.framebuffer,
                    bytes_per_scanline: block.bytes_per_scanline,
                    bytes_per_pixel: block.bits_per_pixel / 8,
                    pixel_format: match (
                        block.red_position,
                        block.green_position,
                        block.blue_position,
                    ) {
                        (0, 8, 16) => PixelFormat::Rgb,
                        (16, 8, 0) => PixelFormat::Bgr,
                        (red_position, green_position, blue_position) => PixelFormat::Unknown {
                            red_position,
                            green_position,
                            blue_position,
                        },
                    },
                    memory_model: block.memory_model,
                    attributes: block.attributes,
                })
            }
            other => Err(other),
        }
    }

    pub fn enable(&self) -> Result<(), u16> {
        let mut ret: u16;
        unsafe {
            asm!(
                "push bx",
                "mov bx, {:x}",
                "int 0x10",
                "pop bx",
                in(reg) self.mode,
                inout("ax") 0x4f02u16 => ret,
            )
        };
        match ret {
            0x4f => Ok(()),
            other => Err(other),
        }
    }
}
