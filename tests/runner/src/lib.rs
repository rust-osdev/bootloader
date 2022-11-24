use std::{io::Write, path::Path, process::Command};

const QEMU_ARGS: &[&str] = &[
    "-device",
    "isa-debug-exit,iobase=0xf4,iosize=0x04",
    "-serial",
    "stdio",
    // "-display",
    // "none",
    "--no-reboot",
];

pub fn run_test_kernel(kernel_binary_path: &str) {
    let kernel_path = Path::new(kernel_binary_path);

    // create an MBR disk image for legacy BIOS booting
    let mbr_path = kernel_path.with_extension("mbr");
    bootloader::BiosBoot::new(kernel_path)
        .create_disk_image(&mbr_path)
        .unwrap();

    // create a GPT disk image for UEFI booting
    let gpt_path = kernel_path.with_extension("gpt");
    let uefi_builder = bootloader::UefiBoot::new(kernel_path);
    uefi_builder.create_disk_image(&gpt_path).unwrap();

    // create a TFTP folder with the kernel executable and UEFI bootloader for
    // UEFI PXE booting
    let tftp_path = kernel_path.with_extension(".tftp");
    uefi_builder.create_pxe_tftp_folder(&tftp_path).unwrap();

    run_test_kernel_on_uefi(&gpt_path);
    run_test_kernel_on_bios(&mbr_path);
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
