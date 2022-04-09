/*!
An experimental x86_64 bootloader that works on both BIOS and UEFI systems.

To use this crate, specify it as a dependency in the `Cargo.toml` of your operating system
kernel. Then you can use the [`entry_point`] macro to mark your entry point function. This
gives you access to the [`BootInfo`] struct, which is passed by the bootloader.

## Disk Image Creation

Including the `bootloader` crate as a dependency makes the kernel binary suitable for booting,
but does not create any bootable disk images. To create them, two additional steps are needed:

1. **Locate the source code of the `bootloader` dependency** on your local system. By using the
   dependency source code directly, we ensure that the kernel and bootloader use the same version
   of the [`BootInfo`] struct.
    - When creating a builder binary written in Rust, the
      [`bootloader_locator`](https://docs.rs/bootloader-locator/0.0.4/bootloader_locator/) crate can
      be used to automate this step.
    - Otherwise, the
      [`cargo metadata`](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html) subcommand
      can be used to locate the dependency. The command outputs a JSON object with various metadata
      for the current package. To find the `bootloader` source path in it, first look for the
      "bootloader" dependency under `resolve.nodes.deps` to find out its ID (in the `pkg` field).
      Then use that ID to find the bootloader in `packages`. Its `manifest_path` field contains the
      local path to the `Cargo.toml` of the bootloader.
2. **Run the following command** in the source code directory of the `bootloader` dependency to create
   the bootable disk images:

    ```notrust
    cargo builder --kernel-manifest path/to/kernel/Cargo.toml --kernel-binary path/to/kernel_bin
    ```

    The `--kernel-manifest` argument should point to the `Cargo.toml` of your kernel. It is used
    for applying configuration settings. The `--kernel-binary` argument should point to the kernel
    executable that should be used for the bootable disk images.

    In addition to the `--kernel-manifest` and `--kernel-binary` arguments, it is recommended to also
    set the `--target-dir` and `--out-dir` arguments. The former specifies the directory that should
    used for cargo build artifacts and the latter specfies the directory where the resulting disk
    images should be placed. It is recommended to set `--target-dir` to the `target` folder of your
    kernel and `--out-dir` to the the parent folder of `--kernel-binary`.

This will result in the following files, which are placed in the specified `--out-dir`:

- A disk image suitable for BIOS booting, named `boot-bios-<kernel>.img`, where `<kernel>` is the
  name of your kernel executable. This image can be started in QEMU or booted on a real machine
  after burning it to an USB stick..
- A disk image suitable for UEFI booting, named `boot-uefi-<kernel>.img`. Like the BIOS disk image,
  this can be started in QEMU (requires OVMF) and burned to an USB stick to run it on a real
  machine.
- Intermediate UEFI files
  - A FAT partition image named `boot-uefi-<kernel>.fat`, which can be directly started in QEMU
    or written as an EFI system partition to a GPT-formatted disk.
  - An EFI file named `boot-uefi-<kernel>.efi`. This executable is the combination of the
    bootloader and kernel executables. It can be started in QEMU or used to construct a bootable
    disk image: Create an EFI system partition formatted with the FAT filesystem and place the
    EFI file under `efi\boot\bootx64.efi` on that filesystem.

**You can find some examples that implement the above steps [in our GitHub repo](https://github.com/rust-osdev/bootloader/tree/main/examples).**

## Configuration

The bootloader can be configured through a `[package.metadata.bootloader]` table in the
`Cargo.toml` of the kernel (the one passed as `--kernel-manifest`). See the [`Config`] struct
for all possible configuration options.
*/

#![warn(missing_docs)]

use std::{
    fs::{self, File},
    io::{self, Seek},
    path::Path,
};

use anyhow::Context;

pub fn create_uefi_disk_image(
    kernel_binary: &Path,
    out_fat_path: &Path,
    out_gpt_path: &Path,
) -> anyhow::Result<()> {
    let bootloader_path = Path::new(env!("UEFI_BOOTLOADER_PATH"));

    create_fat_filesystem(bootloader_path, kernel_binary, &out_fat_path)
        .context("failed to create UEFI FAT filesystem")?;
    create_gpt_disk(out_fat_path, out_gpt_path);

    Ok(())
}

fn create_fat_filesystem(
    bootloader_efi_file: &Path,
    kernel_binary: &Path,
    out_fat_path: &Path,
) -> anyhow::Result<()> {
    const MB: u64 = 1024 * 1024;

    // retrieve size of `.efi` file and round it up
    let efi_size = fs::metadata(&bootloader_efi_file).unwrap().len();
    let kernel_size = fs::metadata(&kernel_binary)
        .context("failed to read metadata of kernel binary")?
        .len();

    // create new filesystem image file at the given path and set its length
    let fat_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&out_fat_path)
        .unwrap();
    let fat_size = efi_size + kernel_size;
    let fat_size_padded_and_rounded = ((fat_size + 1024 * 64 - 1) / MB + 1) * MB;
    fat_file.set_len(fat_size_padded_and_rounded).unwrap();

    // create new FAT file system and open it
    let label = {
        if let Some(name) = bootloader_efi_file.file_stem() {
            let converted = name.to_string_lossy();
            let name = converted.as_bytes();
            let mut label = [0u8; 11];
            let name = &name[..label.len()];
            let slice = &mut label[..name.len()];
            slice.copy_from_slice(name);
            label
        } else {
            *b"MY_RUST_OS!"
        }
    };
    let format_options = fatfs::FormatVolumeOptions::new().volume_label(label);
    fatfs::format_volume(&fat_file, format_options).context("Failed to format UEFI FAT file")?;
    let filesystem = fatfs::FileSystem::new(&fat_file, fatfs::FsOptions::new())
        .context("Failed to open FAT file system of UEFI FAT file")?;

    // copy EFI file to FAT filesystem
    let root_dir = filesystem.root_dir();
    root_dir.create_dir("efi").unwrap();
    root_dir.create_dir("efi/boot").unwrap();
    let mut bootx64 = root_dir.create_file("efi/boot/bootx64.efi").unwrap();
    bootx64.truncate().unwrap();
    io::copy(
        &mut fs::File::open(&bootloader_efi_file).unwrap(),
        &mut bootx64,
    )
    .unwrap();

    // copy kernel to FAT filesystem
    let mut kernel_file = root_dir.create_file("kernel-x86_64")?;
    kernel_file.truncate()?;
    io::copy(&mut fs::File::open(&kernel_binary)?, &mut kernel_file)
        .context("failed to copy kernel to UEFI FAT filesystem")?;

    Ok(())
}

fn create_gpt_disk(fat_image: &Path, out_gpt_path: &Path) {
    // create new file
    let mut disk = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&out_gpt_path)
        .unwrap();

    // set file size
    let partition_size: u64 = fs::metadata(&fat_image).unwrap().len();
    let disk_size = partition_size + 1024 * 64; // for GPT headers
    disk.set_len(disk_size).unwrap();

    // create a protective MBR at LBA0 so that disk is not considered
    // unformatted on BIOS systems
    let mbr = gpt::mbr::ProtectiveMBR::with_lb_size(
        u32::try_from((disk_size / 512) - 1).unwrap_or(0xFF_FF_FF_FF),
    );
    mbr.overwrite_lba0(&mut disk).unwrap();

    // create new GPT structure
    let block_size = gpt::disk::LogicalBlockSize::Lb512;
    let mut gpt = gpt::GptConfig::new()
        .writable(true)
        .initialized(false)
        .logical_block_size(block_size)
        .create_from_device(Box::new(&mut disk), None)
        .unwrap();
    gpt.update_partitions(Default::default()).unwrap();

    // add new EFI system partition and get its byte offset in the file
    let partition_id = gpt
        .add_partition("boot", partition_size, gpt::partition_types::EFI, 0, None)
        .unwrap();
    let partition = gpt.partitions().get(&partition_id).unwrap();
    let start_offset = partition.bytes_start(block_size).unwrap();

    // close the GPT structure and write out changes
    gpt.write().unwrap();

    // place the FAT filesystem in the newly created partition
    disk.seek(io::SeekFrom::Start(start_offset)).unwrap();
    io::copy(&mut File::open(&fat_image).unwrap(), &mut disk).unwrap();
}

// Provides a function to turn a bootloader executable into a disk image.
//
// Used by the `builder` binary. Only available when the `builder` feature is enabled.
// pub mod disk_image;
