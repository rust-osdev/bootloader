use std::{
    env, fs,
    path::Path,
    process::{exit, Command},
};

type ExitCode = i32;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        eprintln!("too many arguments passed: {:?}", args);
        exit(1);
    }
    if args.len() < 2 {
        eprintln!("not enough arguments passed: {:?}", args);
        exit(1);
    }
    let file_path = Path::new(&args[1]);
    if !file_path.exists() {
        eprintln!("file does not exist: {:?}", file_path);
        exit(1);
    }

    match runner(file_path) {
        Err(err) => {
            eprintln!("ERROR: {:?}", err);
            exit(1);
        }
        Ok(Some(exit_code)) => exit(exit_code),
        Ok(None) => {}
    }
}

fn runner(file_path: &Path) -> anyhow::Result<Option<ExitCode>> {
    let uefi_partition_dir = Path::new("target/uefi_esp");
    let boot_dir = uefi_partition_dir.join("EFI").join("BOOT");
    fs::create_dir_all(&boot_dir)?;
    fs::copy(file_path, boot_dir.join("BootX64.efi"))?;

    let ovmf_code = Path::new("ovmf/OVMF_CODE.fd").canonicalize()?;
    let ovmf_vars = Path::new("ovmf/OVMF_VARS.fd").canonicalize()?;

    let mut qemu = Command::new("qemu-system-x86_64");
    qemu.arg("-drive").arg(format!(
        "if=pflash,format=raw,file={},readonly=on",
        ovmf_code.display()
    ));
    qemu.arg("-drive").arg(format!(
        "if=pflash,format=raw,file={},readonly=on",
        ovmf_vars.display()
    ));
    qemu.arg("-drive").arg(format!(
        "format=raw,file=fat:rw:{}",
        uefi_partition_dir.canonicalize()?.display()
    ));
    qemu.arg("-s");
    qemu.arg("-nodefaults");
    qemu.arg("-vga").arg("std");
    qemu.arg("-d").arg("int");
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
