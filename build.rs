// TODO - Make this code less awful
use llvm_tools::{exe, LlvmTools};
use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:warning=Compiling...");
    let out_dir = env::var("OUT_DIR").unwrap();
    let llvm_tools = LlvmTools::new().expect("LLVM tools not found");
    let objcopy = llvm_tools
        .tool(&exe("llvm-objcopy"))
        .expect("llvm-objcopy not found");

    /*build_subproject(
        Path::new("src/real/bootsector"),
        &[
            "_start",
            "real_mode_println",
            "no_int13h_extensions",
            "dap_load_failed",
        ],
        "x86_64-bootsector.json",
        &out_dir,
        &objcopy,
    );*/

    println!("cargo:warning=Compiling stage2...");

    build_subproject(
        Path::new("src/real/stage_2"),
        &[
            "second_stage",
        ],
        "x86_64-stage_2.json",
        &out_dir,
        &objcopy,
    );

}

fn build_subproject(
    subproject_dir: &Path,
    global_symbols: &[&str],
    target: &str,
    out_dir: &str,
    objcopy: &Path,
) {
    let dir_name = subproject_dir.file_name().unwrap().to_str().unwrap();
    let out_dir_path = Path::new(&out_dir);

    let target_dir = out_dir_path.join("..").join("..").join("..").join("..").join("..");

    assert!(
        global_symbols.len() > 0,
        "must have at least one global symbol"
    );

    // build
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&subproject_dir);

    cmd.arg("build").arg("--release").arg("-Zbuild-std=core");    
    cmd.arg("--verbose");

    cmd.arg(format!(
        "--target={}",
        target
    ));

    cmd.arg("-Zunstable-options");
    cmd.arg("--out-dir").arg(&out_dir);
    cmd.arg("--target-dir")
        .arg(&target_dir);
    cmd.env_remove("RUSTFLAGS");
    cmd.env(
        "XBUILD_SYSROOT_PATH",
        out_dir_path.join("target").join(dir_name).join("sysroot"),
    );

    println!("cargo:warning=Out Dir - {}", &out_dir_path.display());
    println!("cargo:warning=Subproject Dir - {}", &subproject_dir.display());
    println!("cargo:warning=Dir Name - {}", &dir_name);
    println!("cargo:warning=Target Dir - {}", &target_dir.display());

    let status = cmd.status().unwrap();
    assert!(status.success(), "Subcrate build failed!");

    // localize symbols
    let mut cmd = Command::new(objcopy);
    for symbol in global_symbols {
        cmd.arg("-G").arg(symbol);
    }
    cmd.arg(target_dir.join(format!("lib{}.a", dir_name)));
    let status = cmd.status().unwrap();
    assert!(status.success(), "Objcopy failed!");

    // emit linker flags
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static={}", dir_name);
}
