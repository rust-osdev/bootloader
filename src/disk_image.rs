use std::{io, path::Path, process::Command};
use thiserror::Error;

/// Creates a bootable disk image from the given bootloader executable.
pub fn create_disk_image(
    bootloader_elf_path: &Path,
    output_bin_path: &Path,
) -> Result<(), DiskImageError> {
    let llvm_tools = llvm_tools::LlvmTools::new()?;
    let objcopy = llvm_tools
        .tool(&llvm_tools::exe("llvm-objcopy"))
        .ok_or(DiskImageError::LlvmObjcopyNotFound)?;

    // convert bootloader to binary
    let mut cmd = Command::new(objcopy);
    cmd.arg("-I").arg("elf64-x86-64");
    cmd.arg("-O").arg("binary");
    cmd.arg("--binary-architecture=i386:x86-64");
    cmd.arg(bootloader_elf_path);
    cmd.arg(output_bin_path);
    let output = cmd.output().map_err(|err| DiskImageError::Io {
        message: "failed to execute llvm-objcopy command",
        error: err,
    })?;
    if !output.status.success() {
        return Err(DiskImageError::ObjcopyFailed {
            stderr: output.stderr,
        });
    }

    pad_to_nearest_block_size(output_bin_path)?;
    Ok(())
}

fn pad_to_nearest_block_size(output_bin_path: &Path) -> Result<(), DiskImageError> {
    const BLOCK_SIZE: u64 = 512;
    use std::fs::OpenOptions;
    let file = OpenOptions::new()
        .write(true)
        .open(&output_bin_path)
        .map_err(|err| DiskImageError::Io {
            message: "failed to open boot image",
            error: err,
        })?;
    let file_size = file
        .metadata()
        .map_err(|err| DiskImageError::Io {
            message: "failed to get size of boot image",
            error: err,
        })?
        .len();
    let remainder = file_size % BLOCK_SIZE;
    let padding = if remainder > 0 {
        BLOCK_SIZE - remainder
    } else {
        0
    };
    file.set_len(file_size + padding)
        .map_err(|err| DiskImageError::Io {
            message: "failed to pad boot image to a multiple of the block size",
            error: err,
        })
}

/// Creating the disk image failed.
#[derive(Debug, Error)]
pub enum DiskImageError {
    /// The `llvm-tools-preview` rustup component was not found
    #[error(
        "Could not find the `llvm-tools-preview` rustup component.\n\n\
        You can install by executing `rustup component add llvm-tools-preview`."
    )]
    LlvmToolsNotFound,

    /// There was another problem locating the `llvm-tools-preview` rustup component
    #[error("Failed to locate the `llvm-tools-preview` rustup component: {0:?}")]
    LlvmTools(llvm_tools::Error),

    /// The llvm-tools component did not contain the required `llvm-objcopy` executable
    #[error("Could not find `llvm-objcopy` in the `llvm-tools-preview` rustup component.")]
    LlvmObjcopyNotFound,

    /// The `llvm-objcopy` command failed
    #[error("Failed to run `llvm-objcopy`: {}", String::from_utf8_lossy(.stderr))]
    ObjcopyFailed {
        /// The output of `llvm-objcopy` to standard error
        stderr: Vec<u8>,
    },

    /// An unexpected I/O error occurred
    #[error("I/O error: {message}:\n{error}")]
    Io {
        /// Desciption of the failed I/O operation
        message: &'static str,
        /// The I/O error that occured
        error: io::Error,
    },
}

impl From<llvm_tools::Error> for DiskImageError {
    fn from(err: llvm_tools::Error) -> Self {
        match err {
            llvm_tools::Error::NotFound => DiskImageError::LlvmToolsNotFound,
            other => DiskImageError::LlvmTools(other),
        }
    }
}
