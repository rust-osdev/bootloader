# First Stage: Bootsector

This executable needs to fit into the 512-byte boot sector, so we need to use all kinds of tricks to keep the size down.

## Build Commands

1. `cargo build --profile=stage-1 -Zbuild-std=core --target ../../i386-code16-boot-sector.json -Zbuild-std-features=compiler-builtins-mem`
2. `objcopy -I elf32-i386 -O binary ../../target/i386-code16-boot-sector/stage-1/bootloader-x86_64-bios-boot-sector ../../target/disk_image.img`

To run in QEMU:

- `qemu-system-x86_64 -drive format=raw,file=../../target/disk_image.img`

To print the contents of the ELF file, e.g. for trying to bring the size down:

- `objdump -xsdS -M i8086,intel ../../target/i386-code16-boot-sector/stage-1/bootloader-x86_64-bios-boot-sector`
