# bootloader

[![Build Status](https://travis-ci.org/rust-osdev/bootloader.svg?branch=master)](https://travis-ci.org/rust-osdev/bootloader) [![Join the chat at https://gitter.im/rust-osdev/bootloader](https://badges.gitter.im/rust-osdev/bootloader.svg)](https://gitter.im/rust-osdev/bootloader?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

An experimental x86 bootloader written in Rust and inline assembly.

Written for the [second edition](https://github.com/phil-opp/blog_os/issues/360) of the [Writing an OS in Rust](https://os.phil-opp.com) series.

## Design

TODO

## Requirements

You need a nightly [Rust](https://www.rust-lang.org) compiler and [cargo xbuild](https://github.com/rust-osdev/cargo-xbuild). You also need the `llvm-tools-preview` component, which can be installed through `rustup component add llvm-tools-preview`.

## Build

The simplest way to use the bootloader is in combination with the [bootimage](https://github.com/rust-osdev/bootimage) tool. With the tool installed, you can add a normal cargo dependency on the `bootloader` crate to your kernel and then run `bootimage build` to create a bootable disk image. You can also execute `bootimage run` to run your kernel in [QEMU](https://www.qemu.org/) (needs to be installed).

To compile the bootloader manually, you need to invoke `cargo xbuild` with a `KERNEL` environment variable that points to your kernel executable (in the ELF format):

```
KERNEL=/path/to/your/kernel/target/debug/your_kernel cargo xbuild
```

As an example, you can build the bootloader with example kernel from the `example-kernel` directory with the following commands:

```
cd example-kernel
cargo xbuild
cd ..
KERNEL=example-kernel/target/x86_64-example-kernel/debug/example-kernel cargo xbuild --release
```

This results in a bootloader executable at `target/x86_64-bootloader.json/release/bootloader`. This executable is still an ELF file, which can't be run directly.

## Run

To run the compiled bootloader executable, you need to convert it to a binary file. You can use the `llvm-objcopy` tools that ships with the `llvm-tools-preview` rustup component. The easiest way to use this tool is using [`cargo-binutils`](https://github.com/rust-embedded/cargo-binutils), which can be installed through `cargo install cargo-binutils`. Then you can perform the conversion with the following command:

```
cargo objcopy -- -I elf64-x86-64 -O binary --binary-architecture=i386:x86-64 \
  target/x86_64-bootloader/release/bootloader target/x86_64-bootloader/release/bootloader.bin
```

You can run the `bootloader.bin` file using [QEMU](https://www.qemu.org/):

```
qemu-system-x86_64 -drive format=raw,file=target/x86_64-bootloader/release/bootloader.bin
```

Or burn it to an USB drive to boot it on real hardware:

```
dd if=target/x86_64-bootloader/release/bootloader.bin of=/dev/sdX && sync
```

Where sdX is the device name of your USB stick. **Be careful** to choose the correct device name, because everything on that device is overwritten.

## Features
The bootloader crate can be configured through some cargo features:

- `vga_320x200`: This feature switches the VGA hardware to mode 0x13, a graphics mode with resolution 320x200 and 256 colors per pixel. The framebuffer is linear and lives at address `0xa0000`.
- `recursive_page_table`: Maps the level 4 page table recursively and adds the [`recursive_page_table_address`](https://docs.rs/bootloader/0.4.0/bootloader/bootinfo/struct.BootInfo.html#structfield.recursive_page_table_addr) field to the passed `BootInfo`.
- `map_physical_memory`: Maps the complete physical memory in the virtual address space and passes a [`physical_memory_offset`](https://docs.rs/bootloader/0.4.0/bootloader/bootinfo/struct.BootInfo.html#structfield.physical_memory_offset) field in the `BootInfo`.
