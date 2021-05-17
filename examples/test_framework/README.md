# Test Framework Example

This examples showcases how kernels can implement unit and integration testing using the `bootloader` crate.

## Structure

The kernel code is in `src/main.rs`. It requires some special build instructions to recompile the `core` library for the custom target defined in `x86_64-custom.json`. It depends on the `bootloader` crate for booting and [uses the unstable `custom_test_frameworks`](https://os.phil-opp.com/testing/#custom-test-frameworks) feature.

The `boot` sub-crate is responsible for combining the kernel with the bootloader to create bootable disk images. It is configured as a [custom _runner_](https://doc.rust-lang.org/cargo/reference/config.html#targettriplerunner), which means that cargo will automatically invoke it on `cargo run` and `cargo test`. The compiled kernel will hereby be passed as an argument.

## Build Commands

The `.cargo/config.toml` file defines command aliases for the common commands:

- To build the kernel, run **`cargo kbuild`**.
- To build the kernel and turn it into a bootable disk image, run **`cargo kimage`** (short for "kernel image"). This will invoke our `boot` sub-crate with an additional `--no-run` argument so that it just creates the disk image and exits.
- To additionally run the kernel in QEMU after creating the disk image, run **`cargo krun`**.
- To run the unit tests in QEMU, run **`cargo ktest`**.
