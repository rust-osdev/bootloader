#[cfg(not(feature = "binary"))]
fn main() {}

#[cfg(feature = "binary")]
#[derive(Default)]
struct BootloaderConfig {
    physical_memory_offset: Option<u64>,
    kernel_stack_address: Option<u64>,
    kernel_stack_size: Option<u64>,
    boot_info_address: Option<u64>,
}

#[cfg(feature = "binary")]
fn parse_aligned_addr(key: &str, value: &str) -> u64 {
    let num = if value.starts_with("0x") {
        u64::from_str_radix(&value[2..], 16)
    } else {
        u64::from_str_radix(&value, 10)
    };

    let num = num.expect(&format!(
        "`{}` in the kernel manifest must be an integer (is `{}`)",
        key, value
    ));

    if num % 0x1000 != 0 {
        panic!(
            "`{}` in the kernel manifest must be aligned to 4KiB (is `{}`)",
            key, value
        );
    } else {
        num
    }
}

#[cfg(feature = "binary")]
fn parse_to_config(cfg: &mut BootloaderConfig, table: &toml::value::Table) {
    use toml::Value;

    for (key, value) in table {
        match (key.as_str(), value.clone()) {
            ("kernel-stack-address", Value::Integer(i))
            | ("physical-memory-offset", Value::Integer(i))
            | ("boot-info-address", Value::Integer(i)) => {
                panic!(
                    "`{0}` in the kernel manifest must be given as a string, \
                     as toml does not support unsigned 64-bit integers (try `{0} = \"{1}\"`)",
                    key.as_str(),
                    i
                );
            }
            ("kernel-stack-address", Value::String(s)) => {
                cfg.kernel_stack_address = Some(parse_aligned_addr(key.as_str(), &s));
            }
            ("boot-info-address", Value::String(s)) => {
                cfg.boot_info_address = Some(parse_aligned_addr(key.as_str(), &s));
            }
            #[cfg(not(feature = "map_physical_memory"))]
            ("physical-memory-offset", Value::String(_)) => {
                panic!(
                    "`physical-memory-offset` is only supported when the `map_physical_memory` \
                     feature of the crate is enabled"
                );
            }
            #[cfg(feature = "map_physical_memory")]
            ("physical-memory-offset", Value::String(s)) => {
                cfg.physical_memory_offset = Some(parse_aligned_addr(key.as_str(), &s));
            }
            ("kernel-stack-size", Value::Integer(i)) => {
                if i <= 0 {
                    panic!("`kernel-stack-size` in kernel manifest must be positive");
                } else {
                    cfg.kernel_stack_size = Some(i as u64);
                }
            }
            (s, _) => {
                panic!(
                    "unknown key '{}' in kernel manifest \
                     - you may need to update the bootloader crate",
                    s
                );
            }
        }
    }
}

#[cfg(feature = "binary")]
fn main() {
    use std::{
        env,
        fs::{self, File},
        io::Write,
        path::{Path, PathBuf},
        process::{self, Command},
    };
    use toml::Value;

    let target = env::var("TARGET").expect("TARGET not set");
    if Path::new(&target)
        .file_stem()
        .expect("target has no file stem")
        != "x86_64-bootloader"
    {
        panic!("The bootloader must be compiled for the `x86_64-bootloader.json` target.");
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let kernel = PathBuf::from(match env::var("KERNEL") {
        Ok(kernel) => kernel,
        Err(_) => {
            eprintln!(
                "The KERNEL environment variable must be set for building the bootloader.\n\n\
                 If you use `bootimage` for building you need at least version 0.7.0. You can \
                 update `bootimage` by running `cargo install bootimage --force`."
            );
            process::exit(1);
        }
    });
    let kernel_file_name = kernel
        .file_name()
        .expect("KERNEL has no valid file name")
        .to_str()
        .expect("kernel file name not valid utf8");

    // check that the kernel file exists
    assert!(
        kernel.exists(),
        "KERNEL does not exist: {}",
        kernel.display()
    );

    // get access to llvm tools shipped in the llvm-tools-preview rustup component
    let llvm_tools = match llvm_tools::LlvmTools::new() {
        Ok(tools) => tools,
        Err(llvm_tools::Error::NotFound) => {
            eprintln!("Error: llvm-tools not found");
            eprintln!("Maybe the rustup component `llvm-tools-preview` is missing?");
            eprintln!("  Install it through: `rustup component add llvm-tools-preview`");
            process::exit(1);
        }
        Err(err) => {
            eprintln!("Failed to retrieve llvm-tools component: {:?}", err);
            process::exit(1);
        }
    };

    // check that kernel executable has code in it
    let llvm_size = llvm_tools
        .tool(&llvm_tools::exe("llvm-size"))
        .expect("llvm-size not found in llvm-tools");
    let mut cmd = Command::new(llvm_size);
    cmd.arg(&kernel);
    let output = cmd.output().expect("failed to run llvm-size");
    let output_str = String::from_utf8_lossy(&output.stdout);
    let second_line_opt = output_str.lines().skip(1).next();
    let second_line = second_line_opt.expect("unexpected llvm-size line output");
    let text_size_opt = second_line.split_ascii_whitespace().next();
    let text_size = text_size_opt.expect("unexpected llvm-size output");
    if text_size == "0" {
        panic!("Kernel executable has an empty text section. Perhaps the entry point was set incorrectly?\n\n\
            Kernel executable at `{}`\n", kernel.display());
    }

    // strip debug symbols from kernel for faster loading
    let stripped_kernel_file_name = format!("kernel_stripped-{}", kernel_file_name);
    let stripped_kernel = out_dir.join(&stripped_kernel_file_name);
    let objcopy = llvm_tools
        .tool(&llvm_tools::exe("llvm-objcopy"))
        .expect("llvm-objcopy not found in llvm-tools");
    let mut cmd = Command::new(&objcopy);
    cmd.arg("--strip-debug");
    cmd.arg(&kernel);
    cmd.arg(&stripped_kernel);
    let exit_status = cmd
        .status()
        .expect("failed to run objcopy to strip debug symbols");
    if !exit_status.success() {
        eprintln!("Error: Stripping debug symbols failed");
        process::exit(1);
    }

    // wrap the kernel executable as binary in a new ELF file
    let stripped_kernel_file_name_replaced = stripped_kernel_file_name
        .replace('-', "_")
        .replace('.', "_");
    let kernel_bin = out_dir.join(format!("kernel_bin-{}.o", kernel_file_name));
    let kernel_archive = out_dir.join(format!("libkernel_bin-{}.a", kernel_file_name));
    let mut cmd = Command::new(&objcopy);
    cmd.arg("-I").arg("binary");
    cmd.arg("-O").arg("elf64-x86-64");
    cmd.arg("--binary-architecture=i386:x86-64");
    cmd.arg("--rename-section").arg(".data=.kernel");
    cmd.arg("--redefine-sym").arg(format!(
        "_binary_{}_start=_kernel_start_addr",
        stripped_kernel_file_name_replaced
    ));
    cmd.arg("--redefine-sym").arg(format!(
        "_binary_{}_end=_kernel_end_addr",
        stripped_kernel_file_name_replaced
    ));
    cmd.arg("--redefine-sym").arg(format!(
        "_binary_{}_size=_kernel_size",
        stripped_kernel_file_name_replaced
    ));
    cmd.current_dir(&out_dir);
    cmd.arg(&stripped_kernel_file_name);
    cmd.arg(&kernel_bin);
    let exit_status = cmd.status().expect("failed to run objcopy");
    if !exit_status.success() {
        eprintln!("Error: Running objcopy failed");
        process::exit(1);
    }

    // create an archive for linking
    let ar = llvm_tools
        .tool(&llvm_tools::exe("llvm-ar"))
        .unwrap_or_else(|| {
            eprintln!("Failed to retrieve llvm-ar component");
            eprint!("This component is available since nightly-2019-03-29,");
            eprintln!("so try updating your toolchain if you're using an older nightly");
            process::exit(1);
        });
    let mut cmd = Command::new(ar);
    cmd.arg("crs");
    cmd.arg(&kernel_archive);
    cmd.arg(&kernel_bin);
    let exit_status = cmd.status().expect("failed to run ar");
    if !exit_status.success() {
        eprintln!("Error: Running ar failed");
        process::exit(1);
    }

    // Parse the kernel's Cargo.toml which is given to us by bootimage
    let mut bootloader_config = BootloaderConfig::default();

    match env::var("KERNEL_MANIFEST") {
        Err(env::VarError::NotPresent) => {
            panic!("The KERNEL_MANIFEST environment variable must be set for building the bootloader.\n\n\
                 If you use `bootimage` for building you need at least version 0.7.7. You can \
                 update `bootimage` by running `cargo install bootimage --force`.");
        }
        Err(env::VarError::NotUnicode(_)) => {
            panic!("The KERNEL_MANIFEST environment variable contains invalid unicode")
        }
        Ok(path) => {
            println!("cargo:rerun-if-changed={}", path);

            let contents = fs::read_to_string(&path).expect(&format!(
                "failed to read kernel manifest file (path: {})",
                path
            ));

            let manifest = contents
                .parse::<Value>()
                .expect("failed to parse kernel's Cargo.toml");

            let table = manifest
                .get("package")
                .and_then(|table| table.get("metadata"))
                .and_then(|table| table.get("bootloader"))
                .and_then(|table| table.as_table());

            if let Some(table) = table {
                parse_to_config(&mut bootloader_config, table);
            }
        }
    }

    // Configure constants for the bootloader
    // We leave some variables as Option<T> rather than hardcoding their defaults so that they
    // can be calculated dynamically by the bootloader.
    let file_path = out_dir.join("bootloader_config.rs");
    let mut file = File::create(file_path).expect("failed to create bootloader_config.rs");
    file.write_all(
        format!(
            "const PHYSICAL_MEMORY_OFFSET: Option<u64> = {:?};
            const KERNEL_STACK_ADDRESS: Option<u64> = {:?};
            const KERNEL_STACK_SIZE: u64 = {};
            const BOOT_INFO_ADDRESS: Option<u64> = {:?};",
            bootloader_config.physical_memory_offset,
            bootloader_config.kernel_stack_address,
            bootloader_config.kernel_stack_size.unwrap_or(512), // size is in number of pages
            bootloader_config.boot_info_address,
        )
        .as_bytes(),
    )
    .expect("write to bootloader_config.rs failed");

    // pass link arguments to rustc
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!(
        "cargo:rustc-link-lib=static=kernel_bin-{}",
        kernel_file_name
    );

    println!("cargo:rerun-if-env-changed=KERNEL");
    println!("cargo:rerun-if-env-changed=KERNEL_MANIFEST");
    println!("cargo:rerun-if-changed={}", kernel.display());
    println!("cargo:rerun-if-changed=build.rs");
}
