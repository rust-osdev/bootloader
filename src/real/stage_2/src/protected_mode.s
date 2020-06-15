.intel_syntax noprefix
.code16

protected_mode_switch:
    cli

    mov eax, cr0
    or al, 1
    mov cr0, eax

    push 0x8
    lea eax, [protected_mode]
    push eax
    retf

.code32
protected_mode:
    mov bx, 0x10

    mov ds, bx
    mov es, bx

    jmp third_stage