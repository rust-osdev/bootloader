# rustboot

[![Join the chat at https://gitter.im/rust-osdev/bootloader](https://badges.gitter.im/rust-osdev/bootloader.svg)](https://gitter.im/rust-osdev/bootloader?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

An experimental pure-Rust x86 bootloader for the [planned second edition](https://github.com/phil-opp/blog_os/issues/360) of the [Writing an OS in Rust](https://os.phil-opp.com) series.

**This is still work in progress**.

The idea is to build the kernel as a `no_std` longmode executable and then build the bootloader with the kernel [ELF](https://en.wikipedia.org/wiki/Executable_and_Linkable_Format) file in `kernel.bin`. The output is a flat binary disk image (including a basic [MBR](https://en.wikipedia.org/wiki/Master_boot_record)) that can be run in [QEMU](https://www.qemu.org/) or burned to an USB flash drive (CDs require a different kind of bootloader, which is not supported at the moment). The plan is to create a custom tool (or cargo subcommand) that performs these steps automatically.

## Build and Run
You need a nightly [Rust](https://www.rust-lang.org) compiler, [xargo](https://github.com/japaric/xargo), [objcopy](https://sourceware.org/binutils/docs/binutils/objcopy.html) (or a similar tool), and [QEMU](https://www.qemu.org/) (for running it).

### Mac OS

If you are building on Mac OS and get a error saying `ld.bfd not found` you first need to [cross compile binutils](https://os.phil-opp.com/cross-compile-binutils) and then adjust
the linker name in the `x86_64-bootloader.json` file. The reason for this is the default rust LLVM linker doesn't support some features this project needs.

After doing that continue with the instructions for Linux.

### Linux

```
> RUST_TARGET_PATH=$(pwd) xargo build --target x86_64-bootloader --release
> objcopy -O binary -S target/x86_64-bootloader/release/bootloader bootimage.bin
> qemu-system-x86_64 -hda bootimage.bin -d int -s
```
