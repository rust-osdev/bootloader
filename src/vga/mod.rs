//! Provides access to the VGA driver.

mod vga;
mod vga_colors;
mod vga_configurations;
mod vga_fonts;
mod vga_registers;
mod vga_writers;

pub use vga::{Plane, Vga, VideoMode, VGA};
pub use vga_colors::{Color16Bit, TextModeColor};
pub use vga_writers::{Graphics640x480x16, Text40x25, Text40x50, Text80x25};
