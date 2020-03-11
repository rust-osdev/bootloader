use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

const ST00_READ_ADDRESS: u16 = 0x3C2;
const ST01_READ_CGA_ADDRESS: u16 = 0x3DA;
const ST01_READ_MDA_ADDRESS: u16 = 0x3BA;
const FCR_READ_ADDRESS: u16 = 0x3CA;
const FCR_CGA_WRITE_ADDRESS: u16 = 0x3DA;
const FCR_MDA_WRITE_ADDRESS: u16 = 0x3BA;
const MSR_READ_ADDRESS: u16 = 0x3CC;
const MSR_WRITE_ADDRESS: u16 = 0x3C2;

const SRX_INDEX_ADDRESS: u16 = 0x3C4;
const SRX_DATA_ADDRESS: u16 = 0x3C5;

const GRX_INDEX_ADDRESS: u16 = 0x3CE;
const GRX_DATA_ADDRESS: u16 = 0x3CF;

const ARX_INDEX_ADDRESS: u16 = 0x3C0;
const ARX_DATA_ADDRESS: u16 = 0x3C1;

const CRX_INDEX_CGA_ADDRESS: u16 = 0x3D4;
const CRX_INDEX_MDA_ADDRESS: u16 = 0x3B4;
const CRX_DATA_CGA_ADDRESS: u16 = 0x3D5;
const CRX_DATA_MDA_ADDRESS: u16 = 0x3B5;

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum EmulationMode {
    Mda = 0x0,
    Cga = 0x1,
}

impl From<u8> for EmulationMode {
    fn from(value: u8) -> EmulationMode {
        match value {
            0x0 => EmulationMode::Mda,
            0x1 => EmulationMode::Cga,
            _ => panic!("{} is an invalid emulation mode", value),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum SequencerIndex {
    SequencerReset = 0x0,
    ClockingMode = 0x1,
    PlaneMask = 0x2,
    CharacterFont = 0x3,
    MemoryMode = 0x4,
    CounterReset = 0x7,
}

impl From<SequencerIndex> for u8 {
    fn from(value: SequencerIndex) -> u8 {
        value as u8
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum GraphicsControllerIndex {
    SetReset = 0x0,
    EnableSetReset = 0x1,
    ColorCompare = 0x2,
    DataRotate = 0x3,
    ReadPlaneSelect = 0x4,
    GraphicsMode = 0x5,
    Miscellaneous = 0x6,
    ColorDontCare = 0x7,
    BitMask = 0x8,
    AddressMapping = 0x10,
    PageSelector = 0x11,
    SoftwareFlags = 0x18,
}

impl From<GraphicsControllerIndex> for u8 {
    fn from(value: GraphicsControllerIndex) -> u8 {
        value as u8
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum AttributeControllerIndex {
    PaletteRegister0 = 0x00,
    PaletteRegister1 = 0x01,
    PaletteRegister2 = 0x02,
    PaletteRegister3 = 0x03,
    PaletteRegister4 = 0x04,
    PaletteRegister5 = 0x05,
    PaletteRegister6 = 0x06,
    PaletteRegister7 = 0x07,
    PaletteRegister8 = 0x08,
    PaletteRegister9 = 0x09,
    PaletteRegisterA = 0x0A,
    PaletteRegisterB = 0x0B,
    PaletteRegisterC = 0x0C,
    PaletteRegisterD = 0x0D,
    PaletteRegisterE = 0x0E,
    PaletteRegisterF = 0x0F,
    ModeControl = 0x10,
    OverscanColor = 0x11,
    MemoryPlaneEnable = 0x12,
    HorizontalPixelPanning = 0x13,
    ColorSelect = 0x14,
}

impl From<AttributeControllerIndex> for u8 {
    fn from(value: AttributeControllerIndex) -> u8 {
        value as u8
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum CrtcControllerIndex {
    HorizontalTotal = 0x00,
    HorizontalDisplayEnableEnd = 0x01,
    HorizontalBlankingStart = 0x02,
    HorizontalBlankingEnd = 0x03,
    HorizontalSyncStart = 0x04,
    HorizontalSyncEnd = 0x05,
    VeritcalTotal = 0x06,
    Overflow = 0x07,
    PresetRowScan = 0x08,
    MaximumScanLine = 0x09,
    TextCursorStart = 0x0A,
    TextCursorEnd = 0x0B,
    StartAddressHigh = 0x0C,
    StartAddressLow = 0x0D,
    TextCursorLocationHigh = 0x0E,
    TextCursorLocationLow = 0x0F,
    VerticalSyncStart = 0x10,
    VerticalSyncEnd = 0x11,
    VerticalDisplayEnableEnd = 0x12,
    Offset = 0x13,
    UnderlineLocationRegister = 0x14,
    VerticalBlankingStart = 0x15,
    VerticalBlankingEnd = 0x16,
    ModeControl = 0x17,
    LineCompare = 0x18,
    MemoryReadLatchData = 0x22,
    ToggleStateOfAttributeController = 0x24,
}

impl From<CrtcControllerIndex> for u8 {
    fn from(value: CrtcControllerIndex) -> u8 {
        value as u8
    }
}

#[derive(Debug)]
pub struct GeneralRegisters {
    st00_read: PortReadOnly<u8>,
    st01_read_cga: PortReadOnly<u8>,
    st01_read_mda: PortReadOnly<u8>,
    fcr_read: PortReadOnly<u8>,
    fcr_write_cga: PortWriteOnly<u8>,
    fcr_write_mda: PortWriteOnly<u8>,
    msr_read: PortReadOnly<u8>,
    msr_write: PortWriteOnly<u8>,
}

impl GeneralRegisters {
    pub fn new() -> GeneralRegisters {
        GeneralRegisters {
            st00_read: PortReadOnly::new(ST00_READ_ADDRESS),
            st01_read_cga: PortReadOnly::new(ST01_READ_CGA_ADDRESS),
            st01_read_mda: PortReadOnly::new(ST01_READ_MDA_ADDRESS),
            fcr_read: PortReadOnly::new(FCR_READ_ADDRESS),
            fcr_write_cga: PortWriteOnly::new(FCR_CGA_WRITE_ADDRESS),
            fcr_write_mda: PortWriteOnly::new(FCR_MDA_WRITE_ADDRESS),
            msr_read: PortReadOnly::new(MSR_READ_ADDRESS),
            msr_write: PortWriteOnly::new(MSR_WRITE_ADDRESS),
        }
    }

    pub fn read_msr(&mut self) -> u8 {
        unsafe { self.msr_read.read() }
    }

    pub fn write_msr(&mut self, value: u8) {
        unsafe {
            self.msr_write.write(value);
        }
    }
}

#[derive(Debug)]
pub struct SequencerRegisters {
    srx_index: Port<u8>,
    srx_data: Port<u8>,
}

impl SequencerRegisters {
    pub fn new() -> SequencerRegisters {
        SequencerRegisters {
            srx_index: Port::new(SRX_INDEX_ADDRESS),
            srx_data: Port::new(SRX_DATA_ADDRESS),
        }
    }

    pub fn read(&mut self, index: SequencerIndex) -> u8 {
        self.set_index(index);
        unsafe { self.srx_data.read() }
    }

    pub fn write(&mut self, index: SequencerIndex, value: u8) {
        self.set_index(index);
        unsafe {
            self.srx_data.write(value);
        }
    }

    fn set_index(&mut self, index: SequencerIndex) {
        unsafe {
            self.srx_index.write(u8::from(index));
        }
    }
}

#[derive(Debug)]
pub struct GraphicsControllerRegisters {
    grx_index: Port<u8>,
    grx_data: Port<u8>,
}

impl GraphicsControllerRegisters {
    pub fn new() -> GraphicsControllerRegisters {
        GraphicsControllerRegisters {
            grx_index: Port::new(GRX_INDEX_ADDRESS),
            grx_data: Port::new(GRX_DATA_ADDRESS),
        }
    }

    pub fn read(&mut self, index: GraphicsControllerIndex) -> u8 {
        self.set_index(index);
        unsafe { self.grx_data.read() }
    }

    pub fn write(&mut self, index: GraphicsControllerIndex, value: u8) {
        self.set_index(index);
        unsafe {
            self.grx_data.write(value);
        }
    }

    fn set_index(&mut self, index: GraphicsControllerIndex) {
        unsafe {
            self.grx_index.write(u8::from(index));
        }
    }
}

#[derive(Debug)]
pub struct AttributeControllerRegisters {
    arx_index: Port<u8>,
    arx_data: Port<u8>,
    st01_read_cga: Port<u8>,
    st01_read_mda: Port<u8>,
}

impl AttributeControllerRegisters {
    pub fn new() -> AttributeControllerRegisters {
        AttributeControllerRegisters {
            arx_index: Port::new(ARX_INDEX_ADDRESS),
            arx_data: Port::new(ARX_DATA_ADDRESS),
            st01_read_cga: Port::new(ST01_READ_CGA_ADDRESS),
            st01_read_mda: Port::new(ST01_READ_MDA_ADDRESS),
        }
    }

    pub fn write(
        &mut self,
        emulation_mode: EmulationMode,
        index: AttributeControllerIndex,
        value: u8,
    ) {
        self.toggle_index(emulation_mode);
        self.set_index(index);
        unsafe {
            self.arx_index.write(value);
        }
    }

    fn set_index(&mut self, index: AttributeControllerIndex) {
        unsafe {
            self.arx_index.write(u8::from(index));
        }
    }

    fn toggle_index(&mut self, emulation_mode: EmulationMode) {
        let st01_read = match emulation_mode {
            EmulationMode::Cga => &mut self.st01_read_cga,
            EmulationMode::Mda => &mut self.st01_read_mda,
        };
        unsafe {
            st01_read.read();
        }
    }

    /// Video Enable. Note that In the VGA standard, this is called the "Palette Address Source" bit.
    /// Clearing this bit will cause the VGA display data to become all 00 index values. For the default
    /// palette, this will cause a black screen. The video timing signals continue. Another control bit will
    /// turn video off and stop the data fetches.
    ///
    /// 0 = Disable. Attribute controller color registers (AR[00:0F]) can be accessed by the CPU.
    ///
    /// 1 = Enable. Attribute controller color registers (AR[00:0F]) are inaccessible by the CPU.
    pub fn blank_screen(&mut self, emulation_mode: EmulationMode) {
        self.toggle_index(emulation_mode);
        let arx_index_value = unsafe { self.arx_index.read() };
        unsafe {
            self.arx_index.write(arx_index_value & 0xDF);
        }
    }

    /// Video Enable. Note that In the VGA standard, this is called the "Palette Address Source" bit.
    /// Clearing this bit will cause the VGA display data to become all 00 index values. For the default
    /// palette, this will cause a black screen. The video timing signals continue. Another control bit will
    /// turn video off and stop the data fetches.
    ///
    /// 0 = Disable. Attribute controller color registers (AR[00:0F]) can be accessed by the CPU.
    ///
    /// 1 = Enable. Attribute controller color registers (AR[00:0F]) are inaccessible by the CPU.
    pub fn unblank_screen(&mut self, emulation_mode: EmulationMode) {
        self.toggle_index(emulation_mode);
        let arx_index_value = unsafe { self.arx_index.read() };
        unsafe {
            self.arx_index.write(arx_index_value | 0x20);
        }
    }
}

#[derive(Debug)]
pub struct CrtcControllerRegisters {
    crx_index_cga: Port<u8>,
    crx_index_mda: Port<u8>,
    crx_data_cga: Port<u8>,
    crx_data_mda: Port<u8>,
}

impl CrtcControllerRegisters {
    pub fn new() -> CrtcControllerRegisters {
        CrtcControllerRegisters {
            crx_index_cga: Port::new(CRX_INDEX_CGA_ADDRESS),
            crx_index_mda: Port::new(CRX_INDEX_MDA_ADDRESS),
            crx_data_cga: Port::new(CRX_DATA_CGA_ADDRESS),
            crx_data_mda: Port::new(CRX_DATA_MDA_ADDRESS),
        }
    }

    pub fn read(&mut self, emulation_mode: EmulationMode, index: CrtcControllerIndex) -> u8 {
        self.set_index(emulation_mode, index);
        unsafe { self.get_data_port(emulation_mode).read() }
    }

    pub fn write(&mut self, emulation_mode: EmulationMode, index: CrtcControllerIndex, value: u8) {
        self.set_index(emulation_mode, index);
        unsafe {
            self.get_data_port(emulation_mode).write(value);
        }
    }

    fn set_index(&mut self, emulation_mode: EmulationMode, index: CrtcControllerIndex) {
        unsafe {
            self.get_index_port(emulation_mode).write(u8::from(index));
        }
    }

    fn get_data_port(&mut self, emulation_mode: EmulationMode) -> &mut Port<u8> {
        match emulation_mode {
            EmulationMode::Cga => &mut self.crx_data_cga,
            EmulationMode::Mda => &mut self.crx_data_mda,
        }
    }

    fn get_index_port(&mut self, emulation_mode: EmulationMode) -> &mut Port<u8> {
        match emulation_mode {
            EmulationMode::Cga => &mut self.crx_index_cga,
            EmulationMode::Mda => &mut self.crx_index_mda,
        }
    }
}
