use super::{
    vga_configurations::{
        VgaConfiguration, MODE_40X25_CONFIGURATION, MODE_40X50_CONFIGURATION,
        MODE_640X480X16_CONFIGURATION, MODE_80X25_CONFIGURATION,
    },
    vga_fonts::{VgaFont, TEXT_8X16_FONT, TEXT_8X8_FONT},
    vga_registers::{
        AttributeControllerRegisters, CrtcControllerIndex, CrtcControllerRegisters, EmulationMode,
        GeneralRegisters, GraphicsControllerIndex, GraphicsControllerRegisters, SequencerIndex,
        SequencerRegisters,
    },
};
use conquer_once::spin::Lazy;
use spinning_top::Spinlock;

/// Provides mutable access to the static `Vga`.
pub static VGA: Lazy<Spinlock<Vga>> = Lazy::new(|| Spinlock::new(Vga::new()));

#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum FrameBuffer {
    GraphicsMode = 0xa0000,
    CgaMode = 0xb8000,
    MdaMode = 0xb0000,
}

impl From<u8> for FrameBuffer {
    fn from(value: u8) -> FrameBuffer {
        match value {
            0x1 => FrameBuffer::GraphicsMode,
            0x2 => FrameBuffer::MdaMode,
            0x3 => FrameBuffer::CgaMode,
            _ => panic!("{:X} is not a valid FrameBuffer value", value),
        }
    }
}

impl From<FrameBuffer> for u32 {
    fn from(value: FrameBuffer) -> u32 {
        value as u32
    }
}

/// Represents a plane for reading and writing vga data.
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Plane {
    /// Represents `Plane 0 (0x0)`.
    Plane0 = 0x0,
    /// Represents `Plane 1 (0x1)`.
    Plane1 = 0x1,
    /// Represents `Plane 2 (0x2)`.
    Plane2 = 0x2,
    /// Represents `Plane 3 (0x3)`.
    Plane3 = 0x3,
}

impl From<Plane> for u8 {
    fn from(value: Plane) -> u8 {
        value as u8
    }
}

/// Represents a specified vga video mode.
#[derive(Debug, Clone, Copy)]
pub enum VideoMode {
    /// Represents text mode 40x25.
    Mode40x25,
    /// Represents text mode 40x50.
    Mode40x50,
    /// Represents text mode 80x25.
    Mode80x25,
    /// Represents graphics mode 640x480x16.
    Mode640x480x16,
}

/// Represents a vga graphics card with it's common registers,
/// as well as the most recent video mode.
pub struct Vga {
    general_registers: GeneralRegisters,
    sequencer_registers: SequencerRegisters,
    graphics_controller_registers: GraphicsControllerRegisters,
    attribute_controller_registers: AttributeControllerRegisters,
    crtc_controller_registers: CrtcControllerRegisters,
    most_recent_video_mode: Option<VideoMode>,
}

impl Vga {
    fn new() -> Vga {
        Vga {
            general_registers: GeneralRegisters::new(),
            sequencer_registers: SequencerRegisters::new(),
            graphics_controller_registers: GraphicsControllerRegisters::new(),
            attribute_controller_registers: AttributeControllerRegisters::new(),
            crtc_controller_registers: CrtcControllerRegisters::new(),
            most_recent_video_mode: None,
        }
    }

    /// Sets the vga graphics card to the given `VideoMode`.
    pub fn set_video_mode(&mut self, video_mode: VideoMode) {
        match video_mode {
            VideoMode::Mode40x25 => self.set_video_mode_40x25(),
            VideoMode::Mode40x50 => self.set_video_mode_40x50(),
            VideoMode::Mode80x25 => self.set_video_mode_80x25(),
            VideoMode::Mode640x480x16 => self.set_video_mode_640x480x16(),
        }
    }

    /// Gets the `FrameBuffer` address as specified by the
    /// `Miscellaneous Output Register`.
    pub fn get_frame_buffer(&mut self) -> FrameBuffer {
        let miscellaneous_graphics = self
            .graphics_controller_registers
            .read(GraphicsControllerIndex::Miscellaneous);
        let memory_map_mode = (miscellaneous_graphics >> 0x2) & 0x3;
        FrameBuffer::from(memory_map_mode)
    }

    /// Returns the most recent video mode, or `None` if no
    /// video mode has been set yet.
    pub fn get_most_recent_video_mode(&self) -> Option<VideoMode> {
        self.most_recent_video_mode
    }

    /// `I/O Address Select` Bit 0 `(value & 0x1)` of MSR selects 3Bxh or 3Dxh as the I/O address for the CRT Controller
    /// registers, the Feature Control Register (FCR), and Input Status Register 1 (ST01). Presently
    /// ignored (whole range is claimed), but will "ignore" 3Bx for color configuration or 3Dx for
    /// monochrome.
    /// Note that it is typical in AGP chipsets to shadow this bit and properly steer I/O cycles to the
    /// proper bus for operation where a MDA exists on another bus such as ISA.
    ///
    /// 0 = Select 3Bxh I/O address `(EmulationMode::Mda)`
    ///
    /// 1 = Select 3Dxh I/O address `(EmulationMode:Cga)`
    fn get_emulation_mode(&mut self) -> EmulationMode {
        EmulationMode::from(self.general_registers.read_msr() & 0x1)
    }

    fn load_font(&mut self, vga_font: &VgaFont) {
        // Save registers
        let (
            plane_mask,
            sequencer_memory_mode,
            read_plane_select,
            graphics_mode,
            miscellaneous_graphics,
        ) = self.save_font_registers();

        // Switch to flat addressing
        self.sequencer_registers
            .write(SequencerIndex::MemoryMode, sequencer_memory_mode | 0x04);

        // Disable Even/Odd addressing
        self.graphics_controller_registers
            .write(GraphicsControllerIndex::GraphicsMode, graphics_mode & !0x10);
        self.graphics_controller_registers.write(
            GraphicsControllerIndex::Miscellaneous,
            miscellaneous_graphics & !0x02,
        );

        // Write font to plane
        self.set_plane(Plane::Plane2);

        let frame_buffer = u32::from(self.get_frame_buffer()) as *mut u8;

        for character in 0..vga_font.characters {
            for row in 0..vga_font.character_height {
                let offset = (character * 32) + row;
                let font_offset = (character * vga_font.character_height) + row;
                unsafe {
                    frame_buffer
                        .offset(offset as isize)
                        .write_volatile(vga_font.font_data[font_offset as usize]);
                }
            }
        }

        self.restore_font_registers(
            plane_mask,
            sequencer_memory_mode,
            read_plane_select,
            graphics_mode,
            miscellaneous_graphics,
        );
    }

    fn restore_font_registers(
        &mut self,
        plane_mask: u8,
        sequencer_memory_mode: u8,
        read_plane_select: u8,
        graphics_mode: u8,
        miscellaneous_graphics: u8,
    ) {
        self.sequencer_registers
            .write(SequencerIndex::PlaneMask, plane_mask);
        self.sequencer_registers
            .write(SequencerIndex::MemoryMode, sequencer_memory_mode);
        self.graphics_controller_registers
            .write(GraphicsControllerIndex::ReadPlaneSelect, read_plane_select);
        self.graphics_controller_registers
            .write(GraphicsControllerIndex::GraphicsMode, graphics_mode);
        self.graphics_controller_registers.write(
            GraphicsControllerIndex::Miscellaneous,
            miscellaneous_graphics,
        );
    }

    fn save_font_registers(&mut self) -> (u8, u8, u8, u8, u8) {
        (
            self.sequencer_registers.read(SequencerIndex::PlaneMask),
            self.sequencer_registers.read(SequencerIndex::MemoryMode),
            self.graphics_controller_registers
                .read(GraphicsControllerIndex::ReadPlaneSelect),
            self.graphics_controller_registers
                .read(GraphicsControllerIndex::GraphicsMode),
            self.graphics_controller_registers
                .read(GraphicsControllerIndex::Miscellaneous),
        )
    }

    /// Turns on the given `Plane` in the vga graphics card.
    pub fn set_plane(&mut self, plane: Plane) {
        let mut plane = u8::from(plane);

        plane &= 0x3;

        self.graphics_controller_registers
            .write(GraphicsControllerIndex::ReadPlaneSelect, plane);
        self.sequencer_registers
            .write(SequencerIndex::PlaneMask, 0x1 << plane);
    }

    fn set_registers(&mut self, configuration: &VgaConfiguration) {
        let emulation_mode = self.get_emulation_mode();

        // Set miscellaneous output
        self.general_registers
            .write_msr(configuration.miscellaneous_output);

        // Set the sequencer registers.
        for (index, value) in configuration.sequencer_registers {
            self.sequencer_registers.write(*index, *value);
        }

        // Unlock the crtc registers.
        self.unlock_crtc_registers(emulation_mode);

        // Set the crtc registers.
        for (index, value) in configuration.crtc_controller_registers {
            self.crtc_controller_registers
                .write(emulation_mode, *index, *value);
        }

        // Set the grx registers.
        for (index, value) in configuration.graphics_controller_registers {
            self.graphics_controller_registers.write(*index, *value);
        }

        // Blank the screen so the palette registers are unlocked.
        self.attribute_controller_registers
            .blank_screen(emulation_mode);

        // Set the arx registers.
        for (index, value) in configuration.attribute_controller_registers {
            self.attribute_controller_registers
                .write(emulation_mode, *index, *value);
        }

        // Unblank the screen so the palette registers are locked.
        self.attribute_controller_registers
            .unblank_screen(emulation_mode);
    }

    /// Sets the video card to Mode 40x25.
    fn set_video_mode_40x25(&mut self) {
        self.set_registers(&MODE_40X25_CONFIGURATION);
        self.load_font(&TEXT_8X16_FONT);
        self.most_recent_video_mode = Some(VideoMode::Mode40x25);
    }

    /// Sets the video card to Mode 40x50.
    fn set_video_mode_40x50(&mut self) {
        self.set_registers(&MODE_40X50_CONFIGURATION);
        self.load_font(&TEXT_8X8_FONT);
        self.most_recent_video_mode = Some(VideoMode::Mode40x50);
    }

    /// Sets the video card to Mode 80x25.
    fn set_video_mode_80x25(&mut self) {
        self.set_registers(&MODE_80X25_CONFIGURATION);
        self.load_font(&TEXT_8X16_FONT);
        self.most_recent_video_mode = Some(VideoMode::Mode80x25);
    }

    /// Sets the video card to Mode 640x480x16.
    fn set_video_mode_640x480x16(&mut self) {
        self.set_registers(&MODE_640X480X16_CONFIGURATION);
        self.most_recent_video_mode = Some(VideoMode::Mode640x480x16);
    }

    /// Unlocks the CRTC registers by setting bit 7 to 0 `(value & 0x7F)`.
    ///
    /// `Protect Registers [0:7]`: Note that the ability to write to Bit 4 of the Overflow Register (CR07)
    /// is not affected by this bit (i.e., bit 4 of the Overflow Register is always writeable).
    ///
    /// 0 = Enable writes to registers `CR[00:07]`
    ///
    /// 1 = Disable writes to registers `CR[00:07]`
    fn unlock_crtc_registers(&mut self, emulation_mode: EmulationMode) {
        // Setting bit 7 to 1 used to be required for `VGA`, but says it's
        // ignored in modern hardware. Setting it to 1 just to be safe for older
        // hardware. More information can be found here
        // https://01.org/sites/default/files/documentation/intel-gfx-prm-osrc-hsw-display.pdf
        // under `CR03 - Horizontal Blanking End Register`.
        let horizontal_blanking_end = self
            .crtc_controller_registers
            .read(emulation_mode, CrtcControllerIndex::HorizontalBlankingEnd);
        self.crtc_controller_registers.write(
            emulation_mode,
            CrtcControllerIndex::HorizontalBlankingEnd,
            horizontal_blanking_end | 0x80,
        );

        let vertical_sync_end = self
            .crtc_controller_registers
            .read(emulation_mode, CrtcControllerIndex::VerticalSyncEnd);
        self.crtc_controller_registers.write(
            emulation_mode,
            CrtcControllerIndex::VerticalSyncEnd,
            vertical_sync_end & 0x7F,
        );
    }
}
