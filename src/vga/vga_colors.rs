/// Represents a 16 bit color used for vga display.
#[repr(u8)]
pub enum Color16Bit {
    /// Represents the color `Black (0x0)`.
    Black = 0x0,
    /// Represents the color `Blue (0x1)`.
    Blue = 0x1,
    /// Represents the color `Green (0x2)`.
    Green = 0x2,
    /// Represents the color `Cyan (0x3)`.
    Cyan = 0x3,
    /// Represents the color `Red (0x4)`.
    Red = 0x4,
    /// Represents the color `Magenta (0x5)`.
    Magenta = 0x5,
    /// Represents the color `Brown (0x6)`.
    Brown = 0x6,
    /// Represents the color `LightGrey (0x7)`.
    LightGrey = 0x7,
    /// Represents the color `DarkGrey (0x8)`.
    DarkGrey = 0x8,
    /// Represents the color `LightBlue (0x9)`.
    LightBlue = 0x9,
    /// Represents the color `LightGreen (0xA)`.
    LightGreen = 0xA,
    /// Represents the color `LightCyan (0xB)`.
    LightCyan = 0xB,
    /// Represents the color `LightRed (0xC)`.
    LightRed = 0xC,
    /// Represents the color `Pink (0xD)`.
    Pink = 0xD,
    /// Represents the color `Yellow (0xE)`.
    Yellow = 0xE,
    /// Represents the color `White (0xF)`.
    White = 0xF,
}

/// Represents a color for vga text modes.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct TextModeColor(u8);

impl TextModeColor {
    /// Returns a new `TextModeColor` given the specified `foreground`
    /// and `background` color.
    pub const fn new(foreground: Color16Bit, background: Color16Bit) -> TextModeColor {
        TextModeColor((background as u8) << 4 | (foreground as u8))
    }
}
