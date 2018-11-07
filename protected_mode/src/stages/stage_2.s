.section .second_stage, "awx"
.global second_stage
.intel_syntax noprefix
.code16

# This stage sets the target operating mode, loads the kernel from disk,
# creates an e820 memory map, enters protected mode, and jumps to the
# third stage.

second_stage:
    lea si, second_stage_start_str
    call println

set_target_operating_mode:
    # Some BIOSs assume the processor will only operate in Legacy Mode. We change the Target
    # Operating Mode to "Long Mode Target Only", so the firmware expects each CPU to enter Long Mode
    # once and then stay in it. This allows the firmware to enable mode-specifc optimizations.
    # We save the flags, because CF is set if the callback is not supported (in which case, this is
    # a NOP)
    pushf
    mov ax, 0xec00
    mov bl, 0x2
    int 0x15
    popf

load_kernel_from_disk:
    # start of memory buffer
    lea eax, _kernel_buffer
    mov [dap_buffer_addr], ax

    # number of disk blocks to load
    mov word ptr [dap_blocks], 1

    # number of start block
    lea eax, _kernel_start_addr
    lea ebx, first_stage
    sub eax, ebx
    shr eax, 9 # divide by 512 (block size)
    mov [dap_start_lba], eax

    # destination address
    mov edi, 0x400000

    # block count
    mov ecx, _kib_kernel_size
    add ecx, 511 # align up
    shr ecx, 9

load_next_kernel_block_from_disk:
    # load block from disk
    lea si, dap
    mov ah, 0x42
    int 0x13
    jc kernel_load_failed

    # copy block to 2MiB
    push ecx
    push esi
    mov ecx, 512 / 4
    # move with zero extension
    # because we are moving a word ptr
    # to esi, a 32-bit register.
    movzx esi, word ptr [dap_buffer_addr]
    # move from esi to edi ecx times.
    rep movsd [edi], [esi]
    pop esi
    pop ecx

    # next block
    mov eax, [dap_start_lba]
    add eax, 1
    mov [dap_start_lba], eax

    sub ecx, 1
    jnz load_next_kernel_block_from_disk

create_memory_map:
    lea di, es:[_memory_map]
    call do_e820

enter_protected_mode:
    cli
    lgdt [gdt_32_pointer]

    mov eax, cr0
    or al, 1    # set protected mode bit
    mov cr0, eax

    push 0x8
    lea eax, [protected_mode]
    push eax
    retf

.code32
protected_mode:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    mov eax, 0xb8000
    mov ebx, 0xffffffff
    mov [eax], ebx

    jmp stage_3

spin:
    jmp spin

.code16

kernel_load_failed:
    lea si, kernel_load_failed_str
    jmp error

gdt_32:
    .quad 0x0000000000000000          # Null Descriptor - should be present.
    .quad 0x00CF9A000000FFFF          # 32-bit code descriptor (exec/read).
    .quad 0x00CF92000000FFFF          # 32-bit data descriptor (read/write).

.align 4
    .word 0                           # Padding to make the "address of the GDT" field aligned on a 4-byte boundary

gdt_32_pointer:
    .word gdt_32_pointer - gdt_32 - 1
    .long gdt_32

second_stage_start_str: .asciz "Booting (second stage)..."
loading_kernel_block_str: .asciz "loading kernel block..."
kernel_load_failed_str: .asciz "Failed to load kernel"
no_long_mode_str: .asciz "No long mode support"
no_cpuid_str: .asciz "No CPUID support"
