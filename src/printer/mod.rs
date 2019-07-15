#[cfg(not(feature = "vga_320x200"))]
pub use self::vga_text_80x25::*;

#[cfg(feature = "vga_320x200")]
pub use self::vga_320x200::*;

#[cfg(feature = "vga_320x200")]
mod vga_320x200;

#[cfg(not(feature = "vga_320x200"))]
mod vga_text_80x25;
