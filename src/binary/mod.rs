#[cfg(feature = "uefi_bin")]
pub mod uefi;
#[cfg(feature = "bios_bin")]
pub mod bios;

pub mod legacy_memory_region;