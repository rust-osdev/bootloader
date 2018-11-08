.section .boot, "awx"
.intel_syntax noprefix
.code64

stage_4:
    # call load_elf with kernel start address, size, and memory map as arguments
    movabs rdi, 0x400000 # move absolute 64-bit to register
    mov rsi, _kib_kernel_size
    lea rdx, _memory_map
    movzx rcx, word ptr mmap_ent
    lea r8, __page_table_start
    lea r9, __page_table_end
    lea rax, __bootloader_end
    push rax
    lea rax, __bootloader_start
    push rax
    call load_elf
spin64:
    jmp spin64
