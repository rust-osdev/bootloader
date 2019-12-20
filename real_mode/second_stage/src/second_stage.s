.section .second_stage, "awx"
.global second_stage
.intel_syntax noprefix
.code16

second_stage_asm:
    mov eax, 12345
    ret
