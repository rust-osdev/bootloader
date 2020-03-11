#[repr(u8)]
pub enum Color16Bit {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGrey = 7,
    DarkGrey = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct TextModeColor(u8);

impl TextModeColor {
    pub const fn new(foreground: Color16Bit, background: Color16Bit) -> TextModeColor {
        TextModeColor((background as u8) << 4 | (foreground as u8))
    }
}
