use argh::FromArgs;
use std::{fmt, path::PathBuf, process::Command, str::FromStr};

#[derive(FromArgs)]
/// Build the bootloader
struct BuildArguments {
    /// path to the `Cargo.toml` of the kernel
    #[argh(option)]
    kernel_manifest: PathBuf,

    /// path to the kernel ELF binary
    #[argh(option)]
    kernel_binary: PathBuf,

    /// which firmware interface to build
    #[argh(option, default = "Firmware::All")]
    firmware: Firmware,

    /// whether to run the resulting binary in QEMU
    #[argh(switch)]
    run: bool,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
enum Firmware {
    Bios,
    Uefi,
    All,
}

impl FromStr for Firmware {
    type Err = FirmwareParseError;

    fn from_str(s: &str) -> Result<Self, FirmwareParseError> {
        match s.to_ascii_lowercase().as_str() {
            "bios" => Ok(Firmware::Bios),
            "uefi" => Ok(Firmware::Uefi),
            "all" => Ok(Firmware::All),
            _other => Err(FirmwareParseError),
        }
    }
}

/// Firmware must be one of `uefi`, `bios`, or `all`.
#[derive(Debug, displaydoc::Display, Eq, PartialEq, Copy, Clone)]
struct FirmwareParseError;

fn main() {
    let args: BuildArguments = argh::from_env();

    let build_or_run = if args.run { "run" } else { "build" };

    if args.firmware == Firmware::Uefi || args.firmware == Firmware::All {
        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg(build_or_run).arg("--bin").arg("uefi");
        cmd.arg("--target").arg("x86_64-unknown-uefi");
        cmd.arg("--features").arg("uefi_bin");
        cmd.arg("-Zbuild-std=core");
        cmd.env("KERNEL", &args.kernel_binary);
        cmd.env("KERNEL_MANIFEST", &args.kernel_manifest);
        cmd.status();
    }

    if args.firmware == Firmware::Bios || args.firmware == Firmware::All {
        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg(build_or_run).arg("--bin").arg("bios");
        cmd.arg("--target").arg("x86_64-bootloader.json");
        cmd.arg("--features").arg("bios_bin");
        cmd.arg("-Zbuild-std=core");
        cmd.env("KERNEL", &args.kernel_binary);
        cmd.env("KERNEL_MANIFEST", &args.kernel_manifest);
        cmd.status();
    }
}
