// BIOS Interrupts

/*
#[cfg(target_triple = "i386-unknown-none-code16")]
#[cfg_attr(target_triple = "i386-unknown-none-code16", macro_use)]
pub mod bios;

#[cfg(target_triple = "i386-unknown-none-code16")]
pub use self::bios::*;
*/

// VGA Buffer
//#[cfg(target_triple = "i386-unknown-none-code16")]
//#[cfg_attr(target_triple = "i386-unknown-none-code16", macro_use)]
pub mod vga;

//#[cfg(target_triple = "i386-unknown-none")]
pub use self::vga::*;