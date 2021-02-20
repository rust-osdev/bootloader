// This build script compiles our bootloader. Because of architecture differences we can't use the standard Rust dependency resolution. To get around this (and add some more seperation between different areas) we compile all of the subcrates as static libraries and link them like we would a C dependency

// TODO - Reuse compilation artifacts so core isn't compiled so many times
use llvm_tools::{exe, LlvmTools};
use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // Read environment variables set by cargo
    let cargo_path = env::var("CARGO").expect("Missing CARGO environment variable");
    let cargo = Path::new(&cargo_path);

    let manifest_dir_path =
        env::var("CARGO_MANIFEST_DIR").expect("Missing CARGO_MANIFEST_DIR environment variable");
    let manifest_dir = Path::new(&manifest_dir_path);

    // Find the root project target dir
    let current_dir = env::current_dir().expect("Couldn't get current directory");
    let target_dir_rel = manifest_dir.join("target");
    let target_dir = current_dir.join(target_dir_rel);

    // Find the objcopy binary
    let llvm_tools = LlvmTools::new().expect("LLVM tools not found");
    let objcopy = llvm_tools
        .tool(&exe("llvm-objcopy"))
        .expect("llvm-objcopy not found");

    // Build stage 3
    build_subproject(
        Path::new("src/protected/stage_3"),
        &["third_stage"],
        "../i386-unknown-none.json",
        &target_dir,
        &objcopy,
        &cargo,
    );


    // Build the bootsector
    build_subproject(
        Path::new("src/real/bootsector"),
        &[
            "_start",
            "real_mode_println",
            "no_int13h_extensions",
            "dap_load_failed",
        ],
        "../i386-unknown-none-code16.json",
        &target_dir,
        &objcopy,
        &cargo,
    );

    // Build stage 2
    build_subproject(
        Path::new("src/real/stage_2"),
        &["second_stage", "v8086_test"],
        "../i386-unknown-none-code16.json",
        &target_dir,
        &objcopy,
        &cargo,
    );

    // Inform cargo that we should rerun this on linker script changes
    //
    // This is NOT performed by default
    println!("cargo:rerun-if-changed=linker.ld");
}

fn build_subproject(
    subproject_dir: &Path,
    global_symbols: &[&str],
    target_file_path: &str,
    root_target_dir: &Path,
    objcopy: &Path,
    cargo: &Path,
) {
    let subproject_name = subproject_dir
        .file_stem()
        .expect("Couldn't get subproject name")
        .to_str()
        .expect("Subproject Name is not valid UTF-8");
    let target_file = Path::new(&target_file_path)
        .file_stem()
        .expect("Couldn't get target file stem");

    let target_dir = root_target_dir.join(&subproject_name);

    // We have to export at least 1 symbol
    assert!(
        global_symbols.len() > 0,
        "must have at least one global symbol"
    );

    // Use cargo in CARGO environment variable (set on build)
    let mut build_cmd = Command::new(cargo);

    // Build inside the subproject
    build_cmd.current_dir(&subproject_dir);
    build_cmd.arg("build");

    // Build in release mode if we're built in release mode
    let build_profile = env::var("PROFILE").expect("Couldn't get cargo build profile");

    if build_profile == "release" {
        build_cmd.arg("--release");
    }

    // Very verbose (build script output only shows if you use `-vv` or it fails anyway)
    build_cmd.arg("-vv");

    // Cross-compile core (cargo-xbuild no longer needed)
    build_cmd.arg("-Zbuild-std=core");

    // Use calculated target directory
    build_cmd.arg(format!("--target-dir={}", &target_dir.display()));

    // Use the passed target
    build_cmd.arg("--target").arg(target_file_path);

    // Run the command and make sure it succeeds
    let build_status = build_cmd.status().expect("Subcrate build failed!");
    assert!(build_status.success(), "Subcrate build failed!");

    // Compute the path to the binary
    let binary_dir = target_dir.join(&target_file).join("release");
    let binary_path = binary_dir.join(format!("lib{}.a", &subproject_name));

    // Use passed objcopy
    let mut objcopy_cmd = Command::new(objcopy);

    // Localize all symbols except those passed
    for symbol in global_symbols {
        objcopy_cmd.arg("-G").arg(symbol);
    }

    // Pass the binary as argument
    objcopy_cmd.arg(binary_path);

    // Run the command and make sure it succeeds
    let objcopy_status = objcopy_cmd.status().expect("Objcopy failed!");
    assert!(objcopy_status.success(), "Objcopy failed!");

    // Emit flags to the linker
    //
    // Staticlibs can't be used as normal dependencies, they have to be linked by a build script
    println!("cargo:rustc-link-search=native={}", &binary_dir.display());
    println!("cargo:rustc-link-lib=static={}", &subproject_name);

    // Inform cargo to rerun on source changes
    //
    // Cargo doesn't understand that the subcrates are part of the project because of how we build them, we have to tell it ourselves
    println!("cargo:rerun-if-changed={}", &target_file_path);
    println!("cargo:rerun-if-changed={}", &subproject_dir.display());
}
