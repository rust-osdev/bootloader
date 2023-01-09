use crate::fat;
use anyhow::Context;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

mod mbr;

const BIOS_STAGE_3: &str = "boot-stage-3";
const BIOS_STAGE_4: &str = "boot-stage-4";

/// Create disk images for booting on legacy BIOS systems.
pub struct BiosBoot {
    kernel: PathBuf,
}

impl BiosBoot {
    /// Start creating a disk image for the given bootloader ELF executable.
    pub fn new(kernel_path: &Path) -> Self {
        Self {
            kernel: kernel_path.to_owned(),
        }
    }

    /// Create a bootable UEFI disk image at the given path.
    pub fn create_disk_image(&self, out_path: &Path) -> anyhow::Result<()> {
        let bootsector_path = Path::new(env!("BIOS_BOOT_SECTOR_PATH"));
        let stage_2_path = Path::new(env!("BIOS_STAGE_2_PATH"));

        let fat_partition = self
            .create_fat_partition()
            .context("failed to create FAT partition")?;

        mbr::create_mbr_disk(
            bootsector_path,
            stage_2_path,
            fat_partition.path(),
            out_path,
        )
        .context("failed to create BIOS MBR disk image")?;

        fat_partition
            .close()
            .context("failed to delete FAT partition after disk image creation")?;

        Ok(())
    }

    /// Creates an BIOS-bootable FAT partition with the kernel.
    fn create_fat_partition(&self) -> anyhow::Result<NamedTempFile> {
        let stage_3_path = Path::new(env!("BIOS_STAGE_3_PATH"));
        let stage_4_path = Path::new(env!("BIOS_STAGE_4_PATH"));

        let mut files = BTreeMap::new();
        files.insert(crate::KERNEL_FILE_NAME, self.kernel.as_path());
        files.insert(BIOS_STAGE_3, stage_3_path);
        files.insert(BIOS_STAGE_4, stage_4_path);

        let out_file = NamedTempFile::new().context("failed to create temp file")?;
        fat::create_fat_filesystem(files, out_file.path())
            .context("failed to create BIOS FAT filesystem")?;

        Ok(out_file)
    }
}
