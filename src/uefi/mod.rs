use std::path::Path;

use crate::DiskImageBuilder;

const BIOS_STAGE_3: &str = "boot-stage-3";
const BIOS_STAGE_4: &str = "boot-stage-4";

/// Create disk images for booting on legacy BIOS systems.
pub struct UefiBoot {
    image_builder: DiskImageBuilder
}

impl UefiBoot {
    /// Start creating a disk image for the given bootloader ELF executable.
    pub fn new(kernel_path: &Path) -> Self {
        Self {
            image_builder: DiskImageBuilder::new(kernel_path)
        }
    }

    /// Add a ramdisk file to the image
    pub fn set_ramdisk(&mut self, ramdisk_path: &Path) -> &mut Self {
        self.image_builder.set_ramdisk(ramdisk_path);
        self
    }

    /// Create a bootable UEFI disk image at the given path.
    pub fn create_disk_image(&self, out_path: &Path) -> anyhow::Result<()> {
        self.image_builder.create_uefi_image(out_path)
    }
    
    /// Prepare a folder for use with booting over UEFI_PXE.
    ///
    /// This places the bootloader executable under the path "bootloader". The
    /// DHCP server should set the filename option to that path, otherwise the
    /// bootloader won't be found.
    pub fn create_pxe_tftp_folder(&self, out_path: &Path) -> anyhow::Result<()> {
        self.image_builder.create_uefi_tftp_folder(out_path)
    }
}