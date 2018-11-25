use args::Args;
use byteorder::{ByteOrder, LittleEndian};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
    process,
};

const PROGRAM_NAME: &'static str = "builder";
const PROGRAM_DESC: &'static str = "Builds the bootloader crate.";

const BLOCK_SIZE: usize = 512;
type KernelInfoBlock = [u8; BLOCK_SIZE];

fn main() {
    let mut args = args();

    if help_arg_present() {
        println!("{}", args.full_usage());
        process::exit(0);
    }

    if let Err(args_err) = args.parse_from_cli() {
        writeln!(io::stderr(), "{}", args_err).expect("Failed to write to stderr");
        process::exit(1);
    };

    // load kernel

    let kernel_path: String = args.value_of("kernel").unwrap();
    let kernel_path = Path::new(&kernel_path);
    let mut kernel_file = match File::open(kernel_path) {
        Ok(file) => file,
        Err(err) => {
            writeln!(io::stderr(), "Failed to open kernel at {:?}: {}", kernel_path, err)
                .expect("Failed to write to stderr");
            process::exit(1);
        }
    };
    let kernel_size = kernel_file
        .metadata()
        .map(|m| m.len())
        .unwrap_or_else(|err| {
            writeln!(io::stderr(), "Failed to read size of kernel: {}", err)
                .expect("Failed to write to stderr");
            process::exit(1);
        });
    let kernel_info_block = create_kernel_info_block(kernel_size, None);

    // build bootloader

    let mut build_args = vec![
        "--manifest-path".into(),
        "../Cargo.toml".into(),
        "--target".into(),
        "../x86_64-bootloader.json".into(),
        "--release".into(),
    ];
    if args.value_of("no-default-features").unwrap() {
        build_args.push("--no-default-features".into());
    }
    if args.value_of("all-features").unwrap() {
        build_args.push("--all-features".into());
    }
    if let Some(features) = args.optional_value_of("features").unwrap() {
        build_args.push("--features".into());
        build_args.push(features);
    }

    let exit_status = run_xbuild(&build_args);
    if !exit_status.map(|s| s.success()).unwrap_or(false) {
        process::exit(1)
    }

    let bootloader_elf_path = Path::new("../target/x86_64-bootloader/release/bootloader");
    let mut bootloader_elf_bytes = Vec::new();
    File::open(bootloader_elf_path)
        .and_then(|mut f| f.read_to_end(&mut bootloader_elf_bytes))
        .expect("failed to read bootloader ELF file");

    // read bootloader section of ELF file

    let elf_file = xmas_elf::ElfFile::new(&bootloader_elf_bytes).unwrap();
    xmas_elf::header::sanity_check(&elf_file).unwrap();
    let bootloader_section = elf_file
        .find_section_by_name(".bootloader")
        .expect("bootloader must have a .bootloader section");
    let bootloader_bytes = bootloader_section.raw_data(&elf_file);

    // create output file

    let output_file_path = Path::new("../target/x86_64-bootloader/release/bootimage.bin");

    let mut output_file = File::create(output_file_path).expect("Failed to create output file");
    output_file
        .write_all(bootloader_bytes)
        .expect("Failed to write bootloader bytes to output file");
    output_file
        .write_all(&kernel_info_block)
        .expect("Failed to write kernel info block to output file");

    write_file_to_file(&mut output_file, &mut kernel_file)
        .expect("Failed to write kernel to output file");
    pad_file(&mut output_file, kernel_size as usize, &[0; 512]).expect("Failed to pad file");
}

fn args() -> Args {
    use getopts::Occur;

    let mut args = Args::new(PROGRAM_NAME, PROGRAM_DESC);
    args.flag("h", "help", "Prints the help message");
    args.option(
        "",
        "kernel",
        "Path to the kernel ELF file",
        "KERNEL_PATH",
        Occur::Req,
        None,
    );
    args.option(
        "",
        "features",
        "Space-separated list of features to activate",
        "FEATURES",
        Occur::Optional,
        None,
    );
    args.flag("", "all-features", "Activate all available features");
    args.flag(
        "",
        "no-default-features",
        "Do not activate the `default` feature",
    );
    args
}

fn help_arg_present() -> bool {
    std::env::args()
        .find(|a| a == "--help" || a == "-h")
        .is_some()
}

fn run_xbuild(args: &[String]) -> io::Result<process::ExitStatus> {
    let mut command = process::Command::new("cargo");
    command.arg("xbuild");
    command.args(args);
    let exit_status = command.status()?;

    if !exit_status.success() {
        let mut help_command = process::Command::new("cargo");
        help_command.arg("xbuild").arg("--help");
        help_command.stdout(process::Stdio::null());
        help_command.stderr(process::Stdio::null());
        if let Ok(help_exit_status) = help_command.status() {
            if !help_exit_status.success() {
                let mut stderr = io::stderr();
                writeln!(
                    stderr,
                    "Failed to run `cargo xbuild`. Perhaps it is not installed?"
                )?;
                writeln!(stderr, "Run `cargo install cargo-xbuild` to install it.")?;
            }
        }
    }

    Ok(exit_status)
}

fn create_kernel_info_block(kernel_size: u64, maybe_package_size: Option<u64>) -> KernelInfoBlock {
    let kernel_size = if kernel_size <= u64::from(u32::max_value()) {
        kernel_size as u32
    } else {
        panic!("Kernel can't be loaded by BIOS bootloader because is too big")
    };

    let package_size = if let Some(size) = maybe_package_size {
        if size <= u64::from(u32::max_value()) {
            size as u32
        } else {
            panic!("Package can't be loaded by BIOS bootloader because is too big")
        }
    } else {
        0
    };

    let mut kernel_info_block = [0u8; BLOCK_SIZE];
    LittleEndian::write_u32(&mut kernel_info_block[0..4], kernel_size);
    LittleEndian::write_u32(&mut kernel_info_block[8..12], package_size);

    kernel_info_block
}

fn write_file_to_file(output: &mut File, datafile: &mut File) -> io::Result<()> {
    let data_size = datafile.metadata()?.len();
    let mut buffer = [0u8; 1024];
    let mut acc = 0;
    loop {
        let (n, interrupted) = match datafile.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => (n, false),
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => (0, true),
            Err(e) => Err(e)?,
        };
        if !interrupted {
            acc += n;
            output.write_all(&buffer[..n])?
        }
    }

    assert!(data_size == acc as u64);

    Ok(())
}

fn pad_file(output: &mut File, written_size: usize, padding: &[u8]) -> io::Result<()> {
    let padding_size = (padding.len() - (written_size % padding.len())) % padding.len();
    output.write_all(&padding[..padding_size])
}
