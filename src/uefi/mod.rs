use crate::fat;
use anyhow::Context;
use bootloader_boot_config::BootConfig;
use std::io::Write;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

mod gpt;
mod pxe;

/// Create disk images for booting on UEFI systems.
pub struct UefiBoot {
    kernel: PathBuf,
    ramdisk: Option<PathBuf>,
    config: Option<String>,
}

impl UefiBoot {
    /// Start creating a disk image for the given bootloader ELF executable.
    pub fn new(kernel_path: &Path) -> Self {
        Self {
            kernel: kernel_path.to_owned(),
            ramdisk: None,
            config: None,
        }
    }

    /// Add a ramdisk file to the disk image.
    pub fn set_ramdisk(&mut self, ramdisk_path: &Path) -> &mut Self {
        self.ramdisk = Some(ramdisk_path.to_owned());
        self
    }

    /// Configures the runtime behavior of the bootloader.
    pub fn set_boot_config(&mut self, config: &BootConfig) -> &mut Self {
        self.config = Some(serde_json::to_string(&config).expect("failed to serialize BootConfig"));
        self
    }

    /// Create a bootable UEFI disk image at the given path.
    pub fn create_disk_image(&self, out_path: &Path) -> anyhow::Result<()> {
        let fat_partition = self
            .create_fat_partition()
            .context("failed to create FAT partition")?;

        gpt::create_gpt_disk(fat_partition.path(), out_path)
            .context("failed to create UEFI GPT disk image")?;

        fat_partition
            .close()
            .context("failed to delete FAT partition after disk image creation")?;

        Ok(())
    }

    /// Prepare a folder for use with booting over UEFI_PXE.
    ///
    /// This places the bootloader executable under the path "bootloader". The
    /// DHCP server should set the filename option to that path, otherwise the
    /// bootloader won't be found.
    pub fn create_pxe_tftp_folder(&self, out_path: &Path) -> anyhow::Result<()> {
        let bootloader_path = Path::new(env!("UEFI_BOOTLOADER_PATH"));

        pxe::create_uefi_tftp_folder(
            bootloader_path,
            self.kernel.as_path(),
            self.ramdisk.as_deref(),
            self.config.as_deref(),
            out_path,
        )
        .context("failed to create UEFI PXE tftp folder")?;

        Ok(())
    }

    /// Creates an UEFI-bootable FAT partition with the kernel.
    fn create_fat_partition(&self) -> anyhow::Result<NamedTempFile> {
        let bootloader_path = Path::new(env!("UEFI_BOOTLOADER_PATH"));

        let mut files = BTreeMap::new();
        files.insert("efi/boot/bootx64.efi", bootloader_path);
        files.insert(crate::KERNEL_FILE_NAME, self.kernel.as_path());
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
            .context("failed to create UEFI FAT filesystem")?;

        Ok(out_file)
    }
}
