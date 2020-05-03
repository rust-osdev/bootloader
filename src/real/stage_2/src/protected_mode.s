.intel_syntax noprefix
.code16

protected_mode_switch:
    cli

    lgdt [gdt32info]

    mov eax, cr0
    or al, 1
    mov cr0, eax

    push 0x8
    lea eax, [protected_mode]
    push eax
    retf

protected_mode:
    mov bx, 0x10
    mov ds, bx
    mov es, bx

    jmp third_stage

gdt32info:
   .word gdt32_end - gdt32 - 1  # last byte in table
   .word gdt32                  # start of table

gdt32:
    # entry 0 is always unused
    .quad 0
codedesc:
    .byte 0xff
    .byte 0xff
    .byte 0
    .byte 0
    .byte 0
    .byte 0x9a
    .byte 0xcc
    .byte 0
datadesc:
    .byte 0xff
    .byte 0xff
    .byte 0
    .byte 0
    .byte 0
    .byte 0x92
    .byte 0xcc
    .byte 0
gdt32_end: