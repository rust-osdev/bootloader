use bootloader::BootConfig;
use bootloader::DiskImageBuilder;
use std::path::Path;

#[cfg(feature = "uefi")]
mod omvf;
#[cfg(feature = "uefi")]
use crate::omvf::OvmfPaths;

pub fn run_test_kernel(kernel_binary_path: &str) {
    run_test_kernel_internal(kernel_binary_path, None, None)
}
pub fn run_test_kernel_with_ramdisk(kernel_binary_path: &str, ramdisk_path: Option<&Path>) {
    run_test_kernel_internal(kernel_binary_path, ramdisk_path, None)
}
pub fn run_test_kernel_with_config_file(
    kernel_binary_path: &str,
    config_file: Option<&BootConfig>,
) {
    run_test_kernel_internal(kernel_binary_path, None, config_file)
}

pub fn run_test_kernel_internal(
    kernel_binary_path: &str,
    ramdisk_path: Option<&Path>,
    config_file_path: Option<&BootConfig>,
) {
    let kernel_path = Path::new(kernel_binary_path);
    let mut image_builder = DiskImageBuilder::new(kernel_path.to_owned());
    if let Some(rdp) = ramdisk_path {
        image_builder.set_ramdisk(rdp.to_owned());
    }
    if let Some(cfp) = config_file_path {
        image_builder.set_boot_config(cfp);
    }

    #[cfg(feature = "uefi")]
    {
        let mut ovmf_paths = OvmfPaths::find();
        ovmf_paths.with_temp_vars();

        let gpt_path = kernel_path.with_extension("gpt");
        let tftp_path = kernel_path.with_extension("tftp");
        image_builder.create_uefi_image(&gpt_path).unwrap();
        image_builder.create_uefi_tftp_folder(&tftp_path).unwrap();
        run_test_kernel_on_uefi(&gpt_path, &ovmf_paths);
        run_test_kernel_on_uefi_pxe(&tftp_path, &ovmf_paths);
    }

    #[cfg(feature = "bios")]
    {
        // create an MBR disk image for legacy BIOS booting
        let mbr_path = kernel_path.with_extension("mbr");
        image_builder.create_bios_image(mbr_path.as_path()).unwrap();

        run_test_kernel_on_bios(&mbr_path);
    }
}

#[cfg(feature = "uefi")]
pub fn run_test_kernel_on_uefi(out_gpt_path: &Path, ovmf_paths: &OvmfPaths) {
    let args = [
        "-drive",
        &format!(
            "if=pflash,format=raw,readonly=on,file={}",
            ovmf_paths.code().display()
        ),
        "-drive",
        &format!(
            "if=pflash,format=raw,readonly=off,file={}",
            ovmf_paths.vars().display()
        ),
        "-drive",
        &format!("format=raw,file={}", out_gpt_path.display()),
    ];
    run_qemu(args);
}

#[cfg(feature = "bios")]
pub fn run_test_kernel_on_bios(out_mbr_path: &Path) {
    let args = [
        "-drive",
        &(format!("format=raw,file={}", out_mbr_path.display())),
    ];
    run_qemu(args);
}

#[cfg(feature = "uefi")]
pub fn run_test_kernel_on_uefi_pxe(out_tftp_path: &Path, ovmf_paths: &OvmfPaths) {
    let args = [
        "-netdev",
        &format!(
            "user,id=net0,net=192.168.17.0/24,tftp={},bootfile=bootloader,id=net0",
            out_tftp_path.display()
        ),
        "-device",
        "virtio-net-pci,netdev=net0",
        "-drive",
        &format!(
            "if=pflash,format=raw,readonly=on,file={}",
            ovmf_paths.code().display()
        ),
        "-drive",
        &format!(
            "if=pflash,format=raw,readonly=off,file={}",
            ovmf_paths.vars().display()
        ),
    ];
    run_qemu(args);
}

#[cfg(any(feature = "uefi", feature = "bios"))]
fn run_qemu<'a, A>(args: A)
where
    A: IntoIterator<Item = &'a str>,
{
    use std::{
        io::Read,
        process::{Command, Stdio},
    };

    const QEMU_ARGS: &[&str] = &[
        "-device",
        "isa-debug-exit,iobase=0xf4,iosize=0x04",
        "-serial",
        "stdio",
        "-display",
        "none",
        "--no-reboot",
        "-smp",
        "4",
    ];

    const SEPARATOR: &str = "\n____________________________________\n";

    let mut run_cmd = Command::new("qemu-system-x86_64");
    run_cmd.args(args);
    run_cmd.args(QEMU_ARGS);
    let run_cmd_str = format!("{run_cmd:?}");

    run_cmd.stdout(Stdio::piped());
    run_cmd.stderr(Stdio::piped());
    run_cmd.stdin(Stdio::null());

    let mut child = run_cmd.spawn().unwrap();

    let child_stdout = child.stdout.take().unwrap();
    let mut child_stderr = child.stderr.take().unwrap();

    let copy_stdout = std::thread::spawn(move || {
        let print_cmd = format!("\nRunning {run_cmd_str}\n\n").into_bytes();
        let mut output = print_cmd.chain(child_stdout).chain(SEPARATOR.as_bytes());
        std::io::copy(
            &mut output,
            &mut strip_ansi_escapes::Writer::new(std::io::stdout()),
        )
    });
    let copy_stderr = std::thread::spawn(move || {
        std::io::copy(
            &mut child_stderr,
            &mut strip_ansi_escapes::Writer::new(std::io::stderr()),
        )
    });

    match child.wait().unwrap().code() {
        Some(33) => {} // success
        Some(35) => panic!("Test failed"),
        other => panic!("Test failed with unexpected exit code `{other:?}`"),
    }

    copy_stdout.join().unwrap().unwrap();
    copy_stderr.join().unwrap().unwrap();
}
