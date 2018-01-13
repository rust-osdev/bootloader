# rustboot-x86
An experimental pure-Rust x86 bootloader for the [planned second edition](https://github.com/phil-opp/blog_os/issues/360) of the [Writing an OS in Rust](https://os.phil-opp.com) series.

**This is still work in progress**.

## Build and Run
You need a nightly [Rust](https://www.rust-lang.org) compiler, [xargo](https://github.com/japaric/xargo), [objcopy](https://sourceware.org/binutils/docs/binutils/objcopy.html) (or a similar tool), and [QEMU](https://www.qemu.org/) (for running it).

```
> RUST_TARGET_PATH=(pwd) xargo build --target test
> objcopy -O binary -S target/test/debug/elf_loader test-bin
> qemu-system-x86_64 -hda test-bin -d int -s
```
