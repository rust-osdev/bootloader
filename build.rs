#[cfg(not(feature = "binary"))]
fn main() {}

#[cfg(feature = "binary")]
fn main() {
    use std::{
        env,
        fs::File,
        io::Write,
        path::{Path, PathBuf},
        process::{self, Command},
    };

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
        format!("KERNEL does not exist: {}", kernel.display())
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
    let stripped_kernel_file_name_replaced = stripped_kernel_file_name.replace('-', "_");
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

    // create a file with the `PHYSICAL_MEMORY_OFFSET` constant
    let file_path = out_dir.join("physical_memory_offset.rs");
    let mut file = File::create(file_path).expect("failed to create physical_memory_offset.rs");
    let physical_memory_offset = match env::var("BOOTLOADER_PHYSICAL_MEMORY_OFFSET") {
        Err(env::VarError::NotPresent) => 0o_177777_770_000_000_000_0000u64,
        Err(env::VarError::NotUnicode(_)) => panic!(
            "The `BOOTLOADER_PHYSICAL_MEMORY_OFFSET` environment variable must be valid unicode"
        ),
        Ok(s) => if s.starts_with("0x") {
            u64::from_str_radix(&s[2..], 16)
        } else {
            u64::from_str_radix(&s, 10)
        }
        .expect(&format!(
            "The `BOOTLOADER_PHYSICAL_MEMORY_OFFSET` environment variable must be an integer\
             (is `{}`).",
            s
        )),
    };
    file.write_all(
        format!(
            "const PHYSICAL_MEMORY_OFFSET: u64 = {:#x};",
            physical_memory_offset
        )
        .as_bytes(),
    )
    .expect("write to physical_memory_offset.rs failed");

    // pass link arguments to rustc
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!(
        "cargo:rustc-link-lib=static=kernel_bin-{}",
        kernel_file_name
    );

    println!("cargo:rerun-if-env-changed=KERNEL");
    println!("cargo:rerun-if-env-changed=BOOTLOADER_PHYSICAL_MEMORY_OFFSET");
    println!("cargo:rerun-if-changed={}", kernel.display());
    println!("cargo:rerun-if-changed=build.rs");
}
