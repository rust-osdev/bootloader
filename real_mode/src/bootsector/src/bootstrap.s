.section .bootstrap, "awx"
.global _start
.intel_syntax noprefix
.code16

_start:
    # Zero segment registers
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    # Setup the stack
    lea ebx, _stack_end
    mov esp, ebx

    # Push the drive number as first argument
    push dx

    # Call rust
    call rust_start

spin:
    cli
    hlt
    jmp spin