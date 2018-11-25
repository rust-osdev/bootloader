# bootloader

[![Build Status](https://travis-ci.org/rust-osdev/bootloader.svg?branch=master)](https://travis-ci.org/rust-osdev/bootloader) [![Join the chat at https://gitter.im/rust-osdev/bootloader](https://badges.gitter.im/rust-osdev/bootloader.svg)](https://gitter.im/rust-osdev/bootloader?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

An experimental x86 bootloader written in Rust and inline assembly.

Written for the [second edition](https://github.com/phil-opp/blog_os/issues/360) of the [Writing an OS in Rust](https://os.phil-opp.com) series.

## Design

TODO

## Build and Run
You need a nightly [Rust](https://www.rust-lang.org) compiler and [cargo xbuild](https://github.com/rust-osdev/cargo-xbuild).

Then you can run the `builder` executable with your kernel as argument:

```
cd builder
cargo run -- --kernel path/to/your/kernel/elf/file
```

This will output a file named `bootimage.bin` in the `../target/x86_64-bootloader/release` folder.

You can run this file using [QEMU](https://www.qemu.org/):

```
qemu-system-x86_64 -drive format=raw,file=target/x86_64-bootloader/release/bootimage.bin
```

Or burn it to an USB drive:

```
dd if=target/x86_64-blog_os/debug/bootimage-blog_os.bin of=/dev/sdX && sync
```

Where sdX is the device name of your USB stick. **Be careful** to choose the correct device name, because everything on that device is overwritten.

## Features
The bootloader crate can be configured through some cargo features:

- `vga_320x200`: This feature switches the VGA hardware to mode 0x13, a graphics mode with resolution 320x200 and 256 colors per pixel. The framebuffer is linear and lives at address `0xa0000`.
