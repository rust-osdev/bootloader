# Template: Create a Disk Image

The [`bootloader`](https://docs.rs/bootloader/0.11) crate provides simple functions to create bootable disk images from a kernel. The basic idea is to build your kernel first and then invoke a builder function that calls the disk image creation functions of the `bootloader` crate.

A good way to implement this is to move your kernel into a `kernel` subdirectory. Then you can create 
a new `os` crate at the top level that defines a [workspace](https://doc.rust-lang.org/cargo/reference/workspaces.html). The root package has build-dependencies on the `kernel` [artifact](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#artifact-dependencies) and on the bootloader crate. This allows you to create the bootable disk image in a [cargo build script](https://doc.rust-lang.org/cargo/reference/build-scripts.html) and launch the created image in QEMU in the `main` function.

The files could look like this:

```toml
# .cargo/config.toml

[unstable]
# enable the unstable artifact-dependencies feature, see
# https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#artifact-dependencies
bindeps = true
```

```toml
# Cargo.toml

[package]
name = "os"         # or any other name
version = "0.1.0"

[build-dependencies]
bootloader = "0.11"
test-kernel = { path = "kernel", artifact = "bin", target = "x86_64-unknown-none" }

[dependencies]
# used for UEFI booting in QEMU
ovmf-prebuilt = "0.1.0-alpha.1"

[workspace]
members = ["kernel"]
```

```rust
// build.rs

use std::path::PathBuf;

fn main() {
    // set by cargo, build scripts should use this directory for output files
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    // set by cargo's artifact dependency feature, see
    // https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#artifact-dependencies
    let kernel = PathBuf::from(std::env::var_os("CARGO_BIN_FILE_KERNEL_kernel").unwrap());

    // create an UEFI disk image (optional)
    let uefi_path = out_dir.join("uefi.img");
    bootloader::UefiBoot::new(&kernel).create_disk_image(&uefi_path).unwrap();

    // create a BIOS disk image
    let bios_path = out_dir.join("bios.img");
    bootloader::BiosBoot::new(&kernel).create_disk_image(&bios_path).unwrap();

    // pass the disk image paths as env variables to the `main.rs`
    println!("cargo:rustc-env=UEFI_PATH={}", uefi_path.display());
    println!("cargo:rustc-env=BIOS_PATH={}", bios_path.display());
}
```

```rust
// src/main.rs

fn main() {
    // read env variables that were set in build script
    let uefi_path = env!("UEFI_PATH");
    let bios_path = env!("BIOS_PATH");
    
    // choose whether to start the UEFI or BIOS image
    let uefi = true;

    let mut cmd = std::process::Command::new("qemu-system-x86_64");
    if uefi {
        cmd.arg("-bios").arg(ovmf_prebuilt::ovmf_pure_efi());
        cmd.arg("-drive").arg(format!("format=raw,file={uefi_path}"));
    } else {
        cmd.arg("-drive").arg(format!("format=raw,file={bios_path}"));
    }
    let mut child = cmd.spawn().unwrap();
    child.wait().unwrap();
}
```

Now you should be able to use `cargo build` to create a bootable disk image and `cargo run` to run in QEMU. Your kernel is automatically recompiled when it changes. For more advanced usage, you can add command-line arguments to your `main.rs` to e.g. pass additional arguments to QEMU or to copy the disk images to some path to make it easier to find them (e.g. for copying them to an thumb drive).
