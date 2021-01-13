use anyhow::{anyhow, Context};
use argh::FromArgs;
use bootloader::disk_image::create_disk_image;
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

type ExitCode = i32;

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

    /// suppress stdout output
    #[argh(switch)]
    quiet: bool,

    /// build the bootloader with the given cargo features
    #[argh(option)]
    features: Vec<String>,

    /// use the given path as target directory
    #[argh(option)]
    target_dir: Option<PathBuf>,

    /// place the output binaries at the given path
    #[argh(option)]
    out_dir: Option<PathBuf>,
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

impl Firmware {
    fn uefi(&self) -> bool {
        match self {
            Firmware::Bios => false,
            Firmware::Uefi | Firmware::All => true,
        }
    }

    fn bios(&self) -> bool {
        match self {
            Firmware::Bios | Firmware::All => true,
            Firmware::Uefi => false,
        }
    }
}

/// Firmware must be one of `uefi`, `bios`, or `all`.
#[derive(Debug, displaydoc::Display, Eq, PartialEq, Copy, Clone)]
struct FirmwareParseError;

fn main() -> anyhow::Result<()> {
    let args: BuildArguments = argh::from_env();

    if args.firmware.uefi() {
        let build_or_run = if args.run { "run" } else { "build" };
        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg(build_or_run).arg("--bin").arg("uefi");
        cmd.arg("--target").arg("x86_64-unknown-uefi");
        cmd.arg("--features")
            .arg(args.features.join(" ") + " uefi_bin");
        cmd.arg("-Zbuild-std=core");
        cmd.arg("-Zbuild-std-features=compiler-builtins-mem");
        if let Some(target_dir) = &args.target_dir {
            cmd.arg("--target-dir").arg(target_dir);
        }
        if args.quiet {
            cmd.arg("--quiet");
        }
        cmd.env("KERNEL", &args.kernel_binary);
        cmd.env("KERNEL_MANIFEST", &args.kernel_manifest);
        assert!(cmd.status()?.success());

        // Retrieve binary paths
        cmd.arg("--message-format").arg("json");
        let output = cmd
            .output()
            .context("failed to execute kernel build with json output")?;
        if !output.status.success() {
            return Err(anyhow!("{}", String::from_utf8_lossy(&output.stderr)));
        }
        let mut executables = Vec::new();
        for line in String::from_utf8(output.stdout)
            .context("build JSON output is not valid UTF-8")?
            .lines()
        {
            let mut artifact = json::parse(line).context("build JSON output is not valid JSON")?;
            if let Some(executable) = artifact["executable"].take_string() {
                executables.push(PathBuf::from(executable));
            }
        }

        assert_eq!(executables.len(), 1);
        let executable_path = executables.pop().unwrap();
        let executable_name = executable_path.file_stem().unwrap().to_str().unwrap();
        let kernel_name = args.kernel_binary.file_name().unwrap().to_str().unwrap();

        if let Some(out_dir) = &args.out_dir {
            let efi_file =
                out_dir.join(format!("bootimage-{}-{}.efi", executable_name, kernel_name));
            fs::copy(&executable_path, &efi_file).context("failed to copy efi file to out dir")?;

            let efi_size = fs::metadata(&efi_file)
                .context("failed to read metadata of efi file")?
                .len();

            // create fat partition
            {
                const MB: u64 = 1024 * 1024;

                let fat_path = efi_file.with_extension("fat");
                dbg!(&fat_path);
                let fat_file = fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&fat_path)
                    .context("Failed to create UEFI FAT file")?;
                let efi_size_rounded = ((efi_size - 1) / MB + 1) * MB;
                fat_file
                    .set_len(dbg!(efi_size_rounded))
                    .context("failed to set UEFI FAT file length")?;

                // create new FAT partition
                fatfs::format_volume(&fat_file, fatfs::FormatVolumeOptions::new())
                    .context("Failed to format UEFI FAT file")?;

                // copy EFI file to FAT filesystem
                let partition = fatfs::FileSystem::new(&fat_file, fatfs::FsOptions::new())
                    .context("Failed to open FAT file system of UEFI FAT file")?;
                let root_dir = partition.root_dir();
                root_dir.create_dir("efi")?;
                root_dir.create_dir("efi/boot")?;
                let mut bootx64 = root_dir.create_file("efi/boot/bootx64.efi")?;
                bootx64.truncate()?;
                io::copy(&mut fs::File::open(&executable_path)?, &mut bootx64)?;
            }

            // create gpt disk
            {
                //todo!()
            }
        }
    }

    if args.firmware.bios() {
        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("build").arg("--bin").arg("bios");
        cmd.arg("--profile").arg("release");
        cmd.arg("-Z").arg("unstable-options");
        cmd.arg("--target").arg("x86_64-bootloader.json");
        cmd.arg("--features")
            .arg(args.features.join(" ") + " bios_bin");
        cmd.arg("-Zbuild-std=core");
        cmd.arg("-Zbuild-std-features=compiler-builtins-mem");
        if let Some(target_dir) = &args.target_dir {
            cmd.arg("--target-dir").arg(target_dir);
        }
        if args.quiet {
            cmd.arg("--quiet");
        }
        cmd.env("KERNEL", &args.kernel_binary);
        cmd.env("KERNEL_MANIFEST", &args.kernel_manifest);
        cmd.env("RUSTFLAGS", "-C opt-level=s");
        assert!(cmd.status()?.success());

        // Retrieve binary paths
        cmd.arg("--message-format").arg("json");
        let output = cmd
            .output()
            .context("failed to execute kernel build with json output")?;
        if !output.status.success() {
            return Err(anyhow!("{}", String::from_utf8_lossy(&output.stderr)));
        }
        let mut executables = Vec::new();
        for line in String::from_utf8(output.stdout)
            .context("build JSON output is not valid UTF-8")?
            .lines()
        {
            let mut artifact = json::parse(line).context("build JSON output is not valid JSON")?;
            if let Some(executable) = artifact["executable"].take_string() {
                executables.push(PathBuf::from(executable));
            }
        }

        assert_eq!(executables.len(), 1);
        let executable_path = executables.pop().unwrap();
        let executable_name = executable_path.file_name().unwrap().to_str().unwrap();
        let kernel_name = args.kernel_binary.file_name().unwrap().to_str().unwrap();
        let mut output_bin_path = executable_path
            .parent()
            .unwrap()
            .join(format!("bootimage-{}-{}.bin", executable_name, kernel_name));

        create_disk_image(&executable_path, &output_bin_path)
            .context("Failed to create bootable disk image")?;

        if let Some(out_dir) = &args.out_dir {
            let file = out_dir.join(output_bin_path.file_name().unwrap());
            fs::copy(output_bin_path, &file)?;
            output_bin_path = file;
        }

        if !args.quiet {
            println!(
                "Created bootable disk image at {}",
                output_bin_path.display()
            );
        }

        if args.run {
            bios_run(&output_bin_path)?;
        }
    }

    Ok(())
}

fn bios_run(bin_path: &Path) -> anyhow::Result<Option<ExitCode>> {
    let mut qemu = Command::new("qemu-system-x86_64");
    qemu.arg("-drive")
        .arg(format!("format=raw,file={}", bin_path.display()));
    qemu.arg("-s");
    qemu.arg("--no-reboot");
    println!("{:?}", qemu);
    let exit_status = qemu.status()?;
    let ret = if exit_status.success() {
        None
    } else {
        exit_status.code()
    };
    Ok(ret)
}
