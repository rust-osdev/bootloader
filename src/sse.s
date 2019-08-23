.section .boot, "awx"
.intel_syntax noprefix
.code32

# Checks for SSE support and enables it. Should be loaded after stage 2.
no_sse_msg: .asciz "This system does not support SSE"
no_sse2_msg: .asciz "This system does not support SSE 2"
no_xsave_msg: .asciz "This system does not support XSAVE"

# As a part of the implementation of x86-64, AMD demands a minimum amount of SSE support.
# This function will fail if SSE, SSE2 and XSAVE support are not found together.
enable_sse:
    mov eax, 0x1
    cpuid
    test edx, 1 << 25
    jz .no_sse
    mov eax, 0x1
    cpuid
     test edx, 1 << 26
    jz .no_sse2
    mov eax, 0x1
    cpuid
     test ecx, 1 << 26
    jz .no_xsave
    mov eax, cr0
    # clear coprocessor emulation CR0.EM
    and ax, 0xFFFB
    # set coprocessor monitoring  CR0.MP
    or ax, 0x2
    mov cr0, eax
    mov eax, cr4
    # set CR4.OSFXSR and CR4.OSXMMEXCPT at the same time
    or ax, 3 << 9
    mov cr4, eax
    ret

.no_sse:
    lea si, [no_sse_msg]
    call real_mode_println
.no_sse_spin:
jmp .no_sse_spin

.no_sse2:
    lea si, [no_sse2_msg]
    call real_mode_println
.no_sse2_spin:
jmp .no_sse2_spin

.no_xsave:
    lea si, [no_xsave_msg]
    call real_mode_println
.no_xsave_spin:
jmp .no_xsave_spin
