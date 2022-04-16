use std::{
    path::{Path, PathBuf},
    process::Command,
};

const BOOTLOADER_X86_64_UEFI_VERSION: &str = "0.1.0-alpha.0";
const BOOTLOADER_X86_64_BIOS_BOOT_SECTOR_VERSION: &str = "0.1.0-alpha.0";

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let uefi_path = build_uefi_bootloader(&out_dir);
    println!(
        "cargo:rustc-env=UEFI_BOOTLOADER_PATH={}",
        uefi_path.display()
    );

    let bios_boot_sector_path = build_bios_boot_sector(&out_dir);
    println!(
        "cargo:rustc-env=BIOS_BOOT_SECTOR_PATH={}",
        bios_boot_sector_path.display()
    );
}

fn build_uefi_bootloader(out_dir: &Path) -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut cmd = Command::new(cargo);
    cmd.arg("install").arg("bootloader-x86_64-uefi");
    if Path::new("uefi").exists() {
        // local build
        cmd.arg("--path").arg("uefi");
    } else {
        cmd.arg("--version").arg(BOOTLOADER_X86_64_UEFI_VERSION);
    }
    cmd.arg("--locked");
    cmd.arg("--target").arg("x86_64-unknown-uefi");
    cmd.arg("-Zbuild-std=core")
        .arg("-Zbuild-std-features=compiler-builtins-mem");
    cmd.arg("--root").arg(out_dir);
    cmd.env_remove("RUSTFLAGS");
    let status = cmd
        .status()
        .expect("failed to run cargo install for uefi bootloader");
    if status.success() {
        let path = out_dir.join("bin").join("bootloader-x86_64-uefi.efi");
        assert!(
            path.exists(),
            "uefi bootloader executable does not exist after building"
        );
        path
    } else {
        panic!("failed to build uefi bootloader");
    }
}

fn build_bios_boot_sector(out_dir: &Path) -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut cmd = Command::new(cargo);
    cmd.arg("install").arg("bootloader-x86_64-bios-boot-sector");
    let local_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bios")
        .join("boot_sector");
    if local_path.exists() {
        // local build
        cmd.arg("--path").arg(&local_path);
    } else {
        cmd.arg("--version")
            .arg(BOOTLOADER_X86_64_BIOS_BOOT_SECTOR_VERSION);
    }
    cmd.arg("--locked");
    cmd.arg("--target").arg("x86-16bit.json");
    cmd.arg("--profile").arg("first-stage");
    cmd.arg("-Zbuild-std=core")
        .arg("-Zbuild-std-features=compiler-builtins-mem");
    cmd.arg("--root").arg(out_dir);
    let status = cmd
        .status()
        .expect("failed to run cargo install for bios boot sector");
    if status.success() {
        let path = out_dir
            .join("bin")
            .join("bootloader-x86_64-bios-boot-sector");
        assert!(
            path.exists(),
            "bios boot sector executable does not exist after building"
        );
        path
    } else {
        panic!("failed to build bios boot sector");
    }
}
