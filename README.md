# bootloader

[![Docs](https://docs.rs/bootloader/badge.svg)](https://docs.rs/bootloader)
[![Build Status](https://github.com/rust-osdev/bootloader/actions/workflows/build.yml/badge.svg)](https://github.com/rust-osdev/bootloader/actions/workflows/build.yml)
[![Join the chat at https://gitter.im/rust-osdev/bootloader](https://badges.gitter.im/rust-osdev/bootloader.svg)](https://gitter.im/rust-osdev/bootloader?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

An experimental x86_64 bootloader that works on both BIOS and UEFI systems. Written in Rust and some inline assembly, buildable on all platforms without additional build-time dependencies (just some `rustup` components).

## Requirements

You need a nightly [Rust](https://www.rust-lang.org) compiler with the `llvm-tools-preview` component, which can be installed through `rustup component add llvm-tools-preview`.

## Usage

To make your kernel compatible with `bootloader`:

- Add a dependency on the `bootloader_api` crate in your kernel's `Cargo.toml`.
- Your kernel binary should be `#![no_std]` and `#![no_main]`.
- Define an entry point function with the signature `fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> !`. The function name can be arbitrary.
  - The `boot_info` argument provides information about available memory, the framebuffer, and more. See the API docs for `bootloader_api` crate for details.
- Use the `entry_point` macro to register the entry point function: `bootloader_api::entry_point!(kernel_main);`
  - The macro checks the signature of your entry point function and generates a `_start` entry point symbol for it. (If you use a linker script, make sure that you don't change the entry point name to something else.)
  - To use non-standard configuration, you can pass a second argument of type `&'static bootloader_api::BootloaderConfig` to the `entry_point` macro. For example, you can require a specific stack size for your kernel:
    ```rust
    const CONFIG: bootloader_api::BootloaderConfig = {
        let mut config = bootloader_api::BootloaderConfig::new_default();
        config.kernel_stack_size = 100 * 1024; // 100 KiB
        config
    };
    bootloader_api::entry_point!(kernel_main, config = &CONFIG);
    ```
- Compile your kernel as normal to an ELF executable. The executable will contain a special section with metadata and the serialized config, which will enable the `bootloader` crate to load it.

To combine your kernel with a bootloader and create a bootable disk image, follow these steps:

- Create a new runner crate, e.g. through `cargo new runner --bin`.
- Add the `bootloader` crate as a `dependency` in the `runner/Cargo.toml`.
- In the `main.rs`, invoke the build commands for your kernel.
  - Alternatively, you can set up an [artifact dependency](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#artifact-dependencies) on your kernel, provided that you use a `rustup`-supported target for your kernel:
    ```toml
    [dependencies]
    my-kernel = { path = "..", artifact = "bin", target = "x86_64-unknown-none" }
    ```
- After building your kernel, obtain the path to the kernel executable.
  - When using an artifact dependency, you can retrieve this path using `env!("CARGO_BIN_FILE_MY_KERNEL_my-kernel")`
- Use the `bootloader::create_boot_partition` function to create a bootable FAT partition at some chosen path.
- Use one or multiple `bootloader::create_*_disk_image` functions to transform the bootable FAT partition into a disk image.
  - Use the `bootloader::create_uefi_disk_image` function to create an UEFI-compatible GPT-formatted disk image.
  - Use the `bootloader::create_bios_disk_image` function to create a BIOS-compatible MBR-formatted disk image.

## Architecture

This project is split into three separate entities:

- A [`bootloader_api`](./api) library with the entry point, configuration, and boot info definitions.
  - Kernels should include this library as a normal cargo dependency.
  - The provided `entry_point` macro will encode the configuration settings into a separate ELF section of the compiled kernel executable.
- [BIOS](./bios) and [UEFI](./uefi) binaries that contain the actual bootloader implementation.
  - The implementations share a higher-level [common library](./common).
  - Both implementations load the kernel at runtime from a FAT partition. This FAT partition is created
  - The configuration is read from a special section of the kernel's ELF file, which is created by the `entry_point` macro of teh `bootloader_api` library.
- A `bootloader` library to create bootable disk images that run a given kernel. This library is the top-level crate in this project.
  - The library builds the BIOS and UEFI implementations in the [`build.rs`](./build.rs).
  - It provides functions to create FAT-formatted bootable disk images, based on the compiled BIOS and UEFI bootloaders.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
