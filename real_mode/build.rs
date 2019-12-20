use std::process::Command;
use std::env;
use std::fs::{self, File};
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // first stage
    let mut cmd = Command::new("cargo");
    cmd.arg("xbuild").arg("--release");
    cmd.arg("--manifest-path=first_stage/Cargo.toml");
    cmd.arg("-Z").arg("unstable-options");
    cmd.arg("--out-dir").arg(&out_dir);
    let status = cmd.status().unwrap();
    assert!(status.success());
    
    // second stage
    let mut cmd = Command::new("cargo");
    cmd.arg("xbuild").arg("--release");
    cmd.arg("--manifest-path=second_stage/Cargo.toml");
    cmd.arg("-Z").arg("unstable-options");
    cmd.arg("--out-dir").arg(&out_dir);
    let status = cmd.status().unwrap();
    assert!(status.success());

    let concat_script = Path::new(&out_dir).join("concat.mri");
    fs::write(&concat_script, "
        create libreal_mode.a
        addlib libfirst_stage.a
        addlib libsecond_stage.a
        save
        end
    ").unwrap();

    // concat archives
    let mut cmd = Command::new("ar");
    cmd.arg("-M").stdin(File::open(concat_script).unwrap());
    cmd.current_dir(&out_dir);
    let status = cmd.status().unwrap();
    assert!(status.success());
    
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=real_mode");
}