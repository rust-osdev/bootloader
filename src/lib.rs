/*!
An experimental x86_64 bootloader that works on both BIOS and UEFI systems.
*/

#![warn(missing_docs)]

extern crate alloc;

#[cfg(feature = "bios")]
mod bios;
#[cfg(feature = "uefi")]
mod gpt;
#[cfg(feature = "bios")]
mod mbr;
#[cfg(feature = "uefi")]
mod uefi;

#[cfg(feature = "uefi")]
pub use uefi::UefiBoot;

#[cfg(feature = "bios")]
pub use bios::BiosBoot;

mod fat;
mod file_data_source;

use std::{
    collections::BTreeMap,
    env::temp_dir,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::Context;

use tempfile::NamedTempFile;

use crate::file_data_source::FileDataSource;
pub use bootloader_boot_config::BootConfig;

const KERNEL_FILE_NAME: &str = "kernel-x86_64";
const RAMDISK_FILE_NAME: &str = "ramdisk";
const CONFIG_FILE_NAME: &str = "boot.json";

#[derive(Clone)]
struct DiskImageFile {
    source: FileDataSource,
    destination: String,
}

/// DiskImageBuilder helps create disk images for a specified set of files.
/// It can currently create MBR (BIOS), GPT (UEFI), and TFTP (UEFI) images.
pub struct DiskImageBuilder {
    files: Vec<DiskImageFile>,
}

impl DiskImageBuilder {
    /// Create a new instance of DiskImageBuilder, with the specified kernel.
    pub fn new(kernel: &Path) -> Self {
        let mut obj = Self::empty();
        obj.set_kernel(kernel);
        obj
    }

    /// Create a new, empty instance of DiskImageBuilder
    pub fn empty() -> Self {
        Self { files: Vec::new() }
    }

    /// Add or replace a kernel to be included in the final image.
    pub fn set_kernel(&mut self, path: &Path) -> &mut Self {
        self.add_or_replace_file(FileDataSource::File(path.to_path_buf()), KERNEL_FILE_NAME)
    }

    /// Add or replace a ramdisk to be included in the final image.
    pub fn set_ramdisk(&mut self, path: &Path) -> &mut Self {
        self.add_or_replace_file(FileDataSource::File(path.to_path_buf()), RAMDISK_FILE_NAME)
    }

    /// Configures the runtime behavior of the bootloader.
    pub fn set_boot_config(&mut self, boot_config: &BootConfig) -> &mut Self {
        let json =
            serde_json::to_string_pretty(boot_config).expect("failed to serialize BootConfig");
        let bytes = json.as_bytes();
        self.add_or_replace_file(FileDataSource::Data(bytes.to_vec()), CONFIG_FILE_NAME)
    }

    /// Add or replace arbitrary files.
    pub fn add_or_replace_file(
        &mut self,
        file_data_source: FileDataSource,
        target: &str,
    ) -> &mut Self {
        self.files.insert(
            0,
            DiskImageFile {
                source: file_data_source,
                destination: target.to_string(),
            },
        );
        self
    }
    fn create_fat_filesystem_image(
        &self,
        internal_files: BTreeMap<&str, FileDataSource>,
    ) -> anyhow::Result<NamedTempFile> {
        let mut local_map = BTreeMap::new();

        for f in self.files.as_slice() {
            local_map.insert(f.destination.as_str(), f.source.clone());
        }

        for k in internal_files {
            if let Some(_) = local_map.insert(k.0, k.1) {
                return Err(anyhow::Error::msg(format!(
                    "Attempted to overwrite internal file: {}",
                    k.0
                )));
            }
        }

        let out_file = NamedTempFile::new().context("failed to create temp file")?;
        fat::create_fat_filesystem(local_map, out_file.path())
            .context("failed to create BIOS FAT filesystem")?;

        Ok(out_file)
    }
    #[cfg(feature = "bios")]
    /// Create an MBR disk image for booting on BIOS systems.
    pub fn create_bios_image(&self, image_path: &Path) -> anyhow::Result<()> {
        const BIOS_STAGE_3: &str = "boot-stage-3";
        const BIOS_STAGE_4: &str = "boot-stage-4";
        let bootsector_path = Path::new(env!("BIOS_BOOT_SECTOR_PATH"));
        let stage_2_path = Path::new(env!("BIOS_STAGE_2_PATH"));
        let stage_3_path = Path::new(env!("BIOS_STAGE_3_PATH"));
        let stage_4_path = Path::new(env!("BIOS_STAGE_4_PATH"));
        let mut internal_files = BTreeMap::new();
        internal_files.insert(
            BIOS_STAGE_3,
            FileDataSource::File(stage_3_path.to_path_buf()),
        );
        internal_files.insert(
            BIOS_STAGE_4,
            FileDataSource::File(stage_4_path.to_path_buf()),
        );

        let fat_partition = self
            .create_fat_filesystem_image(internal_files)
            .context("failed to create FAT partition")?;
        mbr::create_mbr_disk(
            bootsector_path,
            stage_2_path,
            fat_partition.path(),
            image_path,
        )
        .context("failed to create BIOS MBR disk image")?;

        fat_partition
            .close()
            .context("failed to delete FAT partition after disk image creation")?;
        Ok(())
    }

    #[cfg(feature = "uefi")]
    /// Create a GPT disk image for booting on UEFI systems.
    pub fn create_uefi_image(&self, image_path: &Path) -> anyhow::Result<()> {
        const UEFI_BOOT_FILENAME: &str = "efi/boot/bootx64.efi";
        let bootloader_path = Path::new(env!("UEFI_BOOTLOADER_PATH"));
        let mut internal_files = BTreeMap::new();
        internal_files.insert(
            UEFI_BOOT_FILENAME,
            FileDataSource::File(bootloader_path.to_path_buf()),
        );
        let fat_partition = self
            .create_fat_filesystem_image(internal_files)
            .context("failed to create FAT partition")?;
        gpt::create_gpt_disk(fat_partition.path(), image_path)
            .context("failed to create UEFI GPT disk image")?;
        fat_partition
            .close()
            .context("failed to delete FAT partition after disk image creation")?;

        Ok(())
    }

    #[cfg(feature = "uefi")]
    /// Create a folder containing the needed files for UEFI TFTP/PXE booting.
    pub fn create_uefi_tftp_folder(&self, tftp_path: &Path) -> anyhow::Result<()> {
        const UEFI_TFTP_BOOT_FILENAME: &str = "bootloader";
        let bootloader_path = Path::new(env!("UEFI_BOOTLOADER_PATH"));
        fs::create_dir_all(tftp_path)
            .with_context(|| format!("failed to create out dir at {}", tftp_path.display()))?;

        let to = tftp_path.join(UEFI_TFTP_BOOT_FILENAME);
        fs::copy(bootloader_path, &to).with_context(|| {
            format!(
                "failed to copy bootloader from {} to {}",
                bootloader_path.display(),
                to.display()
            )
        })?;

        for f in self.files.as_slice() {
            let to = tftp_path.join(f.destination.clone());

            let mut new_file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(to)?;

            f.source.copy_to(&mut new_file)?;
        }

        Ok(())
    }
}
