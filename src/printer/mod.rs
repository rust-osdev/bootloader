#[cfg(feature = "vga_320x200")]
pub use self::vga_320x200::*;

#[cfg(feature = "vesa_800x600")]
pub use self::vesa_800x600::*;

#[cfg(not(any(feature = "vesa_800x600", feature = "vga_320x200")))]
pub use self::vga_text_80x25::*;

mod vga_text_80x25;
mod vga_320x200;
mod vesa_800x600;
