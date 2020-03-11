mod graphics_640x480x16;
mod text_40x25;
mod text_40x50;
mod text_80x25;

use super::vga_colors::TextModeColor;

pub use graphics_640x480x16::Graphics640x480x16;
pub use text_40x25::Text40x25;
pub use text_40x50::Text40x50;
pub use text_80x25::Text80x25;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct ScreenCharacter {
    character: u8,
    color: TextModeColor,
}
