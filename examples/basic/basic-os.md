# basic-os

A minimal os to showcase the usage of bootloader.

## Overview

The top level `basic` crate defines a [workspace](https://doc.rust-lang.org/cargo/reference/workspaces.html).
- The `kernel` crate is a member of that workspace
    ```toml
    # in Cargo.toml
    [workspace]
    members = ["kernel"]
    ```
- An [artifact dependency](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#artifact-dependencies) is used add the `kernel` crate as a `build-dependency`:
    ```toml
    # in Cargo.toml
    [build-dependencies]
    kernel = { path = "kernel", artifact = "bin", target = "x86_64-unknown-none" }
    ```
    Enable the unstable artifact-dependencies feature:
    ```toml
    # .cargo/config.toml
    [unstable]
    bindeps = true
    ```
    Experimental features are only available on the nightly channel:
    ```toml
    # rust-toolchain.toml
    [toolchain]
    channel = "nightly"
    targets = ["x86_64-unknown-none"]
    ```

The `basic` create combines the kernel with `bootloader`, creates a bootable disk image and launches them.
- A [cargo build script](https://doc.rust-lang.org/cargo/reference/build-scripts.html) is used to create a bootable disk image ([`build.rs`](build.rs)).
- `basic` launches the images in QEMU.

See also [basic-kernel.md](kernel/basic-kernel.md) in the `kernel` subdirectory.

## Usage

Install dependencies:
```sh
$ sudo apt update && sudo apt install qemu-system-x86
```
Build:
```sh
$ cargo build
```
Run:
```sh
$ cargo run bios
```
```sh
$ cargo run uefi
```
