# First Stage: Bootsector

This executable needs to fit into the 512-byte boot sector, so we need to use all kinds of tricks to keep the size down.

## Build Commands

1. `cargo build --release -Zbuild-std=core --target x86-16bit.json -Zbuild-std-features=compiler-builtins-mem`
2. `objcopy -I elf32-i386 -O binary target/x86-16bit/release/first_stage target/disk_image.bin

To run in QEMU:

- `qemu-system-x86_64 -drive format=raw,file=target/disk_image.bin`

To print the contents of the ELF file, e.g. for trying to bring the size down:

- `objdump -xsdS -M i8086,intel target/x86-16bit/release/first_stage`
