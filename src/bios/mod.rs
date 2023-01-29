use std::path::Path;

use crate::DiskImageBuilder;

const BIOS_STAGE_3: &str = "boot-stage-3";
const BIOS_STAGE_4: &str = "boot-stage-4";

/// Create disk images for booting on legacy BIOS systems.
pub struct BiosBoot {
    image_builder: DiskImageBuilder
}

impl BiosBoot {
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

    /// Create a bootable BIOS disk image at the given path.
    pub fn create_disk_image(&self, out_path: &Path) -> anyhow::Result<()> {
        self.image_builder.create_bios_image(out_path)
    }
}