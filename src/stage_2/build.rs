use llvm_tools::{exe, LlvmTools};
use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let llvm_tools = LlvmTools::new().expect("LLVM tools not found");
    let objcopy = llvm_tools
        .tool(&exe("llvm-objcopy"))
        .expect("llvm-objcopy not found");

    build_subproject(
        Path::new("../bootsector"),
        &[
            "_start",
            "real_mode_println",
            "no_int13h_extensions",
            "dap_load_failed",
        ],
        "x86_64-real_mode.json",
        &out_dir,
        &objcopy,
    );
}

fn build_subproject(
    dir: &Path,
    global_symbols: &[&str],
    target: &str,
    out_dir: &str,
    objcopy: &Path,
) {
    let dir_name = dir.file_name().unwrap().to_str().unwrap();
    let manifest_path = dir.join("Cargo.toml");
    let out_path = Path::new(&out_dir);
    assert!(
        global_symbols.len() > 0,
        "must have at least one global symbol"
    );

    // build
    let mut cmd = Command::new("cargo");
    cmd.arg("xbuild").arg("--release");
    cmd.arg("--verbose");
    cmd.arg(format!("--manifest-path={}", manifest_path.display()));
    cmd.arg(format!(
        "--target={}",
        dir.join("../..").join(target).display()
    ));
    cmd.arg("-Z").arg("unstable-options");
    cmd.arg("--out-dir").arg(&out_dir);
    cmd.arg("--target-dir")
        .arg(out_path.join("target").join(dir_name));
    cmd.env_remove("RUSTFLAGS");
    cmd.env(
        "XBUILD_SYSROOT_PATH",
        out_path.join("target").join(dir_name).join("sysroot"),
    );
    let status = cmd.status().unwrap();
    assert!(status.success());

    // localize symbols
    let mut cmd = Command::new(objcopy);
    for symbol in global_symbols {
        cmd.arg("-G").arg(symbol);
    }
    cmd.arg(out_path.join(format!("lib{}.a", dir_name)));
    let status = cmd.status().unwrap();
    assert!(status.success());

    // emit linker flags
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static={}", dir_name);
}
