.section .context_switch, "awx"
.code64

# Expected arguments:
# - boot info ptr in `rdi`
# - entry point address in `rsi`
# - stack pointer in `rdx`
context_switch:

    # load stack pointer
    mov rsp, rdx

    # jump to entry point
    jmp rsi
context_switch_spin:
    jmp context_switch_spin
