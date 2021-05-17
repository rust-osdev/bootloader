# Basic Example

This a minimal example how to create a bootable disk image with the `bootloader` crate.

## Structure

The kernel code is in `src/main.rs`. It requires some special build instructions to recompile the `core` library for the custom target defined in `x86_64-custom.json`. It depends on the `bootloader` crate for booting..

The `simple_boot` sub-crate is responsible for combining the kernel with the bootloader to create bootable disk images. It is configured as a [custom _runner_](https://doc.rust-lang.org/cargo/reference/config.html#targettriplerunner), which means that cargo will automatically invoke it on `cargo run`. The compiled kernel will hereby be passed as an argument.

## Build Commands

The `.cargo/config.toml` file defines command aliases for the common commands:

- To build the kernel, run **`cargo kbuild`**.
- To build the kernel and turn it into a bootable disk image, run **`cargo kimage`** (short for "kernel image"). This will invoke our `boot` sub-crate with an additional `--no-run` argument so that it just creates the disk image and exits.
- To additionally run the kernel in QEMU after creating the disk image, run **`cargo krun`**.
