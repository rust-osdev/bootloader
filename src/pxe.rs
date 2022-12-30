use std::path::Path;

use anyhow::Context;

pub fn create_uefi_tftp_folder(
    bootloader_path: &Path,
    kernel_binary: &Path,
    ramdisk_path: Option<&Path>,
    out_path: &Path,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(out_path)
        .with_context(|| format!("failed to create out dir at {}", out_path.display()))?;

    let to = out_path.join("bootloader");
    std::fs::copy(bootloader_path, &to).with_context(|| {
        format!(
            "failed to copy bootloader from {} to {}",
            bootloader_path.display(),
            to.display()
        )
    })?;

    let to = out_path.join("kernel-x86_64");
    std::fs::copy(kernel_binary, &to).with_context(|| {
        format!(
            "failed to copy kernel from {} to {}",
            kernel_binary.display(),
            to.display()
        )
    })?;
    let to = out_path.join("ramdisk");
    if let Some(rp) = ramdisk_path {
        std::fs::copy(rp, &to).with_context(|| {
            format!(
                "failed to copy ramdisk from {} to {}",
                rp.display(),
                to.display()
            )
        })?;
    }

    Ok(())
}
