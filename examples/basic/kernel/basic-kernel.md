# basic-kernel

A minimal kernel to showcase the usage of bootloader.

The kernel:
- Initalizes a serial port on [COM1](https://wiki.osdev.org/Serial_Ports#Port_Addresses)
- Dumps the [`boot_info`](/api/src/info.rs) it received from `bootloader`
- Prints a message
- Exits QEMU with a custom exit code via the `isa-debug-exit` device
