.intel_syntax noprefix
.code32
/* _enter_v8086(eip: u32) */
_enter_v8086:
    # Store EIP
    pop eax

    # gs, fs, ds, es
    xor ebx, ebx

    push ebx
    push ebx
    push ebx
    push ebx

    # ss, esp
    push ebx
    push ebx

    # eflags
    mov ebx, ((1 << 17) | (1 << 1))
    push ebx

    # cs, eip
    xor ebx, ebx

    push ebx
    push eax

    iret

_spin:
    jmp _spin