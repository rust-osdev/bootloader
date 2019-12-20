.section .boot, "awx"
.global _start
.intel_syntax noprefix
.code16

# This stage initializes the stack, enables the A20 line, loads the rest of
# the bootloader from disk, and jumps to stage_2.

_start:
    # zero segment registers
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    # clear the direction flag (e.g. go forward in memory when using
    # instructions like lodsb)
    cld

    # initialize stack
    mov sp, 0x7c00

    call rust_main

spin:
    hlt
    jmp spin
