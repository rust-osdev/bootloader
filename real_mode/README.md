# 16-bit Rust (Experiment)

This is an experiment to translate the 16-bit code of the bootloader from assembly to Rust.

## Building

To build the project, use cargo-xbuild:

```
cargo xbuild --release
```

The BIOS only loads the first 512 bytes of our executable into memory, so the amount of code that this binary can contain is very limited. This is also the reason why this can only be built in release mode.

If the code does not fit into 512 bytes, the linker will throw the following error:

> rust-lld: error: linker.ld:16: unable to move location counter backward for: .bootloader

## Creating a Disk Image

The output of `cargo xbuild` is an ELF binary, which can't be loaded directly by the BIOS. To boot our project, we must therefore convert it into a flat binary first. This works with the following `objcopy` command:

```
objcopy -I elf32-i386 -O binary target/x86-32bit/release/bootloader image.bin
```

This creates a file named `image.bin` in the root folder of the project, which is a bootable disk image.

## Running it in QEMU

To run the disk image in QEMU, execute the following command:

```
qemu-system-x86_64 -drive format=raw,file=image.bin
```
