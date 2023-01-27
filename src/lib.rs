/*!
An experimental x86_64 bootloader that works on both BIOS and UEFI systems.
*/

#![warn(missing_docs)]

#[cfg(feature = "bios")]
mod bios;
mod fat;
#[cfg(feature = "uefi")]
mod uefi;

#[cfg(feature = "bios")]
pub use bios::BiosBoot;

#[cfg(feature = "uefi")]
pub use uefi::UefiBoot;

pub use bootloader_boot_config::BootConfig;

const KERNEL_FILE_NAME: &str = "kernel-x86_64";
const RAMDISK_FILE_NAME: &str = "ramdisk";
const CONFIG_FILE_NAME: &str = "boot.json";
