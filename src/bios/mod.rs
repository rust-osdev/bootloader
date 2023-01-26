use crate::fat;
use anyhow::Context;
use bootloader_boot_config::BootConfig;
use std::io::Write;
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
    ramdisk: Option<PathBuf>,
    config: Option<String>,
}

impl BiosBoot {
    /// Start creating a disk image for the given bootloader ELF executable.
    pub fn new(kernel_path: &Path) -> Self {
        Self {
            kernel: kernel_path.to_owned(),
            ramdisk: None,
            config: None,
        }
    }

    /// Add a ramdisk file to the image.
    pub fn set_ramdisk(&mut self, ramdisk_path: &Path) -> &mut Self {
        self.ramdisk = Some(ramdisk_path.to_owned());
        self
    }

    /// Configures the runtime behavior of the bootloader.
    pub fn set_boot_config(&mut self, config: &BootConfig) -> &mut Self {
        self.config = Some(serde_json::to_string(&config).expect("failed to serialize BootConfig"));
        self
    }

    /// Create a bootable BIOS disk image at the given path.
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
        if let Some(ramdisk_path) = &self.ramdisk {
            files.insert(crate::RAMDISK_FILE_NAME, ramdisk_path);
        }

        let mut config_file: NamedTempFile;

        if let Some(config_ser) = &self.config {
            config_file = NamedTempFile::new()
                .context("failed to create temp file")
                .unwrap();
            writeln!(config_file, "{config_ser}")?;
            files.insert(crate::CONFIG_FILE_NAME, config_file.path());
        }

        let out_file = NamedTempFile::new().context("failed to create temp file")?;
        fat::create_fat_filesystem(files, out_file.path())
            .context("failed to create BIOS FAT filesystem")?;

        Ok(out_file)
    }
}
