use std::{
    process::Command,
    path::Path,
};

fn main() {
    let mut cmd = Command::new("cargo");
    cmd.arg("xbuild").arg("--release");
    cmd.arg("--manifest-path").arg("protected_mode/Cargo.toml");
    cmd.arg("--target").arg("protected_mode/i686-bootloader.json");
    cmd.env("RUSTFLAGS", "");
    cmd.status().unwrap();

    let out_dir = Path::new("protected_mode/target/i686-bootloader/release").canonicalize().unwrap();

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=protected_mode");

    println!("cargo:rerun-if-changed=protected_mode");
}
