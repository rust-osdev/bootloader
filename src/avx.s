.section .boot, "awx"
.intel_syntax noprefix
.code64

# This asm file enables AVX support before the OS starts.
# AVX is not a requirement for an OS to boot.
# This file should be loaded after stage 3 and just before stage 4.
enable_avx:
    push rax
    push rcx
    xor rcx, rcx
    xgetbv
    or eax, 7
    xsetbv
    pop rcx
    pop rax
    ret
