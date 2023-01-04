use std::{io::Write, path::{Path, PathBuf}, process::Command};

pub extern crate lazy_static;
pub extern crate paste;
#[doc(hidden)]
pub use paste::paste;
#[doc(hidden)]
pub use lazy_static::lazy_static;
use rand::Rng;

const QEMU_ARGS: &[&str] = &[
    "-device",
    "isa-debug-exit,iobase=0xf4,iosize=0x04",
    "-serial",
    "stdio",
    "-display",
    "none",
    "--no-reboot",
    "-m",
    "size=2048",
];

pub fn generate_test_image_filename(path: &Path) -> PathBuf {
    let s: String = rand::thread_rng()
    .sample_iter(&rand::distributions::Alphanumeric)
    .take(8)
    .map(char::from)
    .collect();
    path.with_file_name(s)
}

pub fn run_test_kernel_bios(kernel_binary_path: &str, ramdisk_path: Option<&str>) {
    let kernel_path = Path::new(kernel_binary_path);
    let ramdisk_path = match ramdisk_path {
        Some(rdp) => Some(Path::new(rdp)),
        None => None,
    };

    // create an MBR disk image for legacy BIOS booting
    let mbr_path = generate_test_image_filename(kernel_path).with_extension("mbr");
    let mut bios_builder = bootloader::BiosBoot::new(kernel_path);

    // Set ramdisk for test, if supplied.
    if let Some(rdp) = ramdisk_path {
        bios_builder.set_ramdisk(rdp);
    }

    bios_builder.create_disk_image(&mbr_path).unwrap();

    run_test_kernel_on_bios(&mbr_path);
}

pub fn run_test_kernel_uefi(kernel_binary_path: &str, ramdisk_path: Option<&str>) {
    let kernel_path = Path::new(kernel_binary_path);
    let ramdisk_path = match ramdisk_path {
        Some(rdp) => Some(Path::new(rdp)),
        None => None,
    };

    // create a GPT disk image for UEFI booting
    let gpt_path = generate_test_image_filename(kernel_path).with_extension("gpt");
    let mut uefi_builder = bootloader::UefiBoot::new(kernel_path);

    // Set ramdisk for test, if supplied.
    if let Some(rdp) = ramdisk_path {
        uefi_builder.set_ramdisk(rdp);
    }

    uefi_builder.create_disk_image(&gpt_path).unwrap();


    run_test_kernel_on_uefi(&gpt_path);
}

pub fn run_test_kernel(kernel_binary_path: &str, ramdisk_path: Option<&str>) {
    run_test_kernel_uefi(kernel_binary_path, ramdisk_path);
    run_test_kernel_bios(kernel_binary_path, ramdisk_path);
    run_test_kernel_tftp(kernel_binary_path, ramdisk_path);
}


pub fn run_test_kernel_tftp(kernel_binary_path: &str, ramdisk_path: Option<&str>) {
    let kernel_path = Path::new(kernel_binary_path);
    let ramdisk_path = match ramdisk_path {
        Some(rdp) => Some(Path::new(rdp)),
        None => None,
    };

    let mut uefi_builder = bootloader::UefiBoot::new(kernel_path);

    // Set ramdisk for test, if supplied.
    if let Some(rdp) = ramdisk_path {
        uefi_builder.set_ramdisk(rdp);
    }

    // create a TFTP folder with the kernel executable and UEFI bootloader for
    // UEFI PXE booting
    let tftp_path = generate_test_image_filename(kernel_path).with_extension(".tftp");
    uefi_builder.create_pxe_tftp_folder(&tftp_path).unwrap();

    run_test_kernel_on_uefi_pxe(&tftp_path);
}

pub fn run_test_kernel_on_uefi(out_gpt_path: &Path) {
    let mut run_cmd = Command::new("qemu-system-x86_64");
    run_cmd
        .arg("-drive")
        .arg(format!("format=raw,file={}", out_gpt_path.display()));
    run_cmd.args(QEMU_ARGS);
    run_cmd.arg("-bios").arg(ovmf_prebuilt::ovmf_pure_efi());

    let child_output = run_cmd.output().unwrap();
    strip_ansi_escapes::Writer::new(std::io::stderr())
        .write_all(&child_output.stderr)
        .unwrap();
    strip_ansi_escapes::Writer::new(std::io::stderr())
        .write_all(&child_output.stdout)
        .unwrap();

    match child_output.status.code() {
        Some(33) => {}                     // success
        Some(35) => panic!("Test failed"), // success
        other => panic!("Test failed with unexpected exit code `{:?}`", other),
    }
}

pub fn run_test_kernel_on_bios(out_mbr_path: &Path) {
    let mut run_cmd = Command::new("qemu-system-x86_64");
    run_cmd
        .arg("-drive")
        .arg(format!("format=raw,file={}", out_mbr_path.display()));
    run_cmd.args(QEMU_ARGS);

    let child_output = run_cmd.output().unwrap();
    strip_ansi_escapes::Writer::new(std::io::stderr())
        .write_all(&child_output.stderr)
        .unwrap();
    strip_ansi_escapes::Writer::new(std::io::stderr())
        .write_all(&child_output.stdout)
        .unwrap();

    match child_output.status.code() {
        Some(33) => {}                     // success
        Some(35) => panic!("Test failed"), // success
        other => panic!("Test failed with unexpected exit code `{:?}`", other),
    }
}

pub fn run_test_kernel_on_uefi_pxe(out_tftp_path: &Path) {
    let mut run_cmd = Command::new("qemu-system-x86_64");
    run_cmd.arg("-netdev").arg(format!(
        "user,id=net0,net=192.168.17.0/24,tftp={},bootfile=bootloader,id=net0",
        out_tftp_path.display()
    ));
    run_cmd.arg("-device").arg("virtio-net-pci,netdev=net0");
    run_cmd.args(QEMU_ARGS);
    run_cmd.arg("-bios").arg(ovmf_prebuilt::ovmf_pure_efi());

    let child_output = run_cmd.output().unwrap();
    strip_ansi_escapes::Writer::new(std::io::stderr())
        .write_all(&child_output.stderr)
        .unwrap();
    strip_ansi_escapes::Writer::new(std::io::stderr())
        .write_all(&child_output.stdout)
        .unwrap();

    match child_output.status.code() {
        Some(33) => {} // success
        Some(35) => panic!("Test failed"),
        other => panic!("Test failed with unexpected exit code `{:?}`", other),
    }
}

#[macro_export]
/// Creates a series of test functions for a given kernel image to cover bios, uefi, and tftp
/// 
/// define_test!(name, kernel) will generate all 3 tests, with a ramdisk and no-ramdisk variant.
/// define_test!(name, kernel, ramdisk) will generate all 3 tests, with the specified ramdisk
/// define_test!(name, kernel, without_ramdisk_tests) will generate all 3 tests, with only the no-ramdisk variant
 macro_rules! define_test {
    ($test_name: ident, $bin: tt) => (
        $crate::paste! {
            #[test]
            fn [< $test_name _uefi_without_ramdisk >]() {
                $crate::run_test_kernel_uefi(
                    $bin,
                    None
                );
            }

            #[test]
            fn [< $test_name _tftp_without_ramdisk >]() {
                $crate::run_test_kernel_tftp(
                    $bin,
                    None
                );
            }

            #[test]
            fn [< $test_name _bios_without_ramdisk >]() {
                $crate::run_test_kernel_bios(
                    $bin,
                    None
                );
            }


            #[test]
            fn [< $test_name _uefi_with_ramdisk >]() {
                $crate::run_test_kernel_uefi(
                    $bin,
                    Some("tests/ramdisk.txt")
                );
            }

            #[test]
            fn [< $test_name _tftp_with_ramdisk >]() {
                $crate::run_test_kernel_tftp(
                    $bin,
                    Some("tests/ramdisk.txt")
                );
            }

            #[test]
            fn [< $test_name _bios_with_ramdisk >]() {
                $crate::run_test_kernel_bios(
                    $bin,
                    Some("tests/ramdisk.txt")
                );
            }
        }
     );
     ($test_name: ident, $bin:tt, without_ramdisk_tests) => (
        $crate::paste! {
            #[test]
            fn [< $test_name _uefi_without_ramdisk >]() {
                $crate::run_test_kernel_uefi(
                    $bin,
                    None
                );
            }

            #[test]
            fn [< $test_name _tftp_without_ramdisk >]() {
                $crate::run_test_kernel_tftp(
                    $bin,
                    None
                );
            }

            #[test]
            fn [< $test_name _bios_without_ramdisk >]() {
                $crate::run_test_kernel_bios(
                    $bin,
                    None
                );
            }
        }
     );
     ($test_name: ident, $bin: tt, $ramdisk: tt) => (
        $crate::paste! {
            #[test]
            fn [< $test_name _uefi_with_ramdisk >]() {
                $crate::run_test_kernel_uefi(
                    $bin,
                    Some($ramdisk)
                );
            }

            #[test]
            fn [< $test_name _tftp_with_ramdisk >]() {
                $crate::run_test_kernel_tftp(
                    $bin,
                    Some($ramdisk)
                );
            }

            #[test]
            fn [< $test_name _bios_with_ramdisk >]() {
                $crate::run_test_kernel_bios(
                    $bin,
                    Some($ramdisk)
                );
            }
        }
     );
}