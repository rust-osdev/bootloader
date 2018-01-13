# rustboot-x86
An experimental pure-Rust x86 bootloader for the [planned second edition](https://github.com/phil-opp/blog_os/issues/360) of the [Writing an OS in Rust](https://os.phil-opp.com) series.

**This is still work in progress**.

The idea is to build the kernel as a `no_std` longmode executable and then build the bootloader with the kernel [ELF](https://en.wikipedia.org/wiki/Executable_and_Linkable_Format) file in `kernel.bin`. The output is a flat binary disk image (including a basic [MBR](https://en.wikipedia.org/wiki/Master_boot_record)) that can be run in [QEMU](https://www.qemu.org/) or burned to an USB flash drive (CDs require a different kind of bootloader, which is not supported at the moment). The plan is to create a custom tool (or cargo subcommand) that performs these steps automatically.

## Build and Run
You need a nightly [Rust](https://www.rust-lang.org) compiler, [xargo](https://github.com/japaric/xargo), [objcopy](https://sourceware.org/binutils/docs/binutils/objcopy.html) (or a similar tool), and [QEMU](https://www.qemu.org/) (for running it).

```
> RUST_TARGET_PATH=(pwd) xargo build --target test
> objcopy -O binary -S target/test/debug/elf_loader test-bin
> qemu-system-x86_64 -hda test-bin -d int -s
```
