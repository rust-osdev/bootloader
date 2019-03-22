use std::{env, path::{Path, PathBuf}, process::{self, Command}};

fn main() {
    let target = env::var("TARGET").expect("TARGET not set");
    if Path::new(&target)
        .file_stem()
        .expect("target has no file stem")
        != "x86_64-bootloader"
    {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let kernel = PathBuf::from(match env::var("KERNEL") {
        Ok(kernel) => kernel,
        Err(_) => {
            eprintln!(
                "The KERNEL environment variable must be set for building the bootloader."
            );
            process::exit(1);
        }
    });
    let kernel_file_name = kernel.file_name().expect("KERNEL has no valid file name").to_str().expect("kernel file name not valid utf8");
    let kernel_out_path = out_dir.join(format!("kernel_bin-{}.o", kernel_file_name));
    let kernel_archive_path = out_dir.join(format!("libkernel_bin-{}.a", kernel_file_name));


    let bin_dir = BinDir::new();
    let objcopy = bin_dir.tool(&LlvmTool::tool_name("objcopy")).expect("llvm-objcopy not found in llvm-tools");

    let mut cmd = Command::new(objcopy.path());
    cmd.arg("-I").arg("binary");
    cmd.arg("-O").arg("elf64-x86-64");
    cmd.arg("--binary-architecture=i386:x86-64");
    cmd.arg("--rename-section").arg(".data=.kernel");
    cmd.arg("--redefine-sym").arg(format!("_binary_{}_start=_kernel_start_addr", kernel_file_name));
    cmd.arg("--redefine-sym").arg(format!("_binary_{}_end=_kernel_end_addr", kernel_file_name));
    cmd.arg("--redefine-sym").arg(format!("_binary_{}_size=_kernel_size", kernel_file_name));
    cmd.current_dir(kernel.parent().expect("KERNEL has no valid parent dir"));
    cmd.arg(&kernel_file_name);
    cmd.arg(&kernel_out_path);
    let exit_status = cmd.status().expect("failed to run objcopy");
    if !exit_status.success() {
        eprintln!("Error: Running objcopy failed");
        process::exit(1);
    }

    let ar = bin_dir.tool(&LlvmTool::tool_name("ar")).expect("llvm-ar not found in llvm-tools");
    let mut cmd = Command::new(ar.path());
    cmd.arg("crs");
    cmd.arg(&kernel_archive_path);
    cmd.arg(&kernel_out_path);
    let exit_status = cmd.status().expect("failed to run ar");
    if !exit_status.success() {
        eprintln!("Error: Running ar failed");
        process::exit(1);
    }

    println!("cargo:rerun-if-changed={}", kernel.display());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=kernel_bin-{}", kernel_file_name);
}

#[derive(Debug)]
struct BinDir {
    bin_dir: PathBuf,
}

impl BinDir {
    fn new() -> Self {
        let example_tool_name = LlvmTool::tool_name("objdump");
        let output = Command::new("rustc")
            .arg("--print")
            .arg("sysroot")
            .output()
            .expect("failed to print sysroot");
        if !output.status.success() {
            eprintln!("Failed to execute `rustc --print sysroot`");
            eprintln!("Stderr: {}", String::from_utf8(output.stderr).expect("error not valid unicode"));
            process::exit(1);
        }

        let sysroot = PathBuf::from(String::from_utf8(output.stdout).expect("sysroot not valid unicode").trim());

        let rustlib = sysroot.join("lib").join("rustlib");
        for entry in rustlib.read_dir().expect("read_dir on sysroot dir failed") {
            let bin_dir = entry.expect("failed to read sysroot dir entry").path().join("bin");
            let tool_path = bin_dir.join(&example_tool_name);
            if tool_path.exists() {
                return Self { bin_dir };
            }
        }

        eprintln!("Error: llvm-tools not found");
        eprintln!("Maybe the rustup component `llvm-tools-preview` is missing?");
        eprintln!("  Install it through: `rustup component add llvm-tools-preview`");
        process::exit(1);
    }

    fn tool(&self, tool_name: &str) -> Option<LlvmTool> {
        let tool_path = self.bin_dir.join(&tool_name);
        
        if tool_path.exists() {
            Some(LlvmTool {
                name: tool_name.to_owned(),
                path: tool_path,
            })
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct LlvmTool {
    name: String,
    path: PathBuf,
}

impl LlvmTool {
    fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(target_os = "windows")]
    fn tool_name(tool: &str) -> String {
        format!("llvm-{}.exe", tool)
    }

    #[cfg(not(target_os = "windows"))]
    fn tool_name(tool: &str) -> String {
        format!("llvm-{}", tool)
    }
}
