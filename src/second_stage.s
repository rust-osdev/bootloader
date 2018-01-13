.section .second_stage, "awx"
.intel_syntax noprefix
.code16

loading_kernel_block_str: .asciz "loading kernel block..."
kernel_load_failed_str: .asciz "Failed to load kernel"
no_long_mode_str: .asciz "No long mode support"

second_stage:
    lea si, second_stage_start_str
    call println

load_kernel_from_disk:
    # start of memory buffer
    lea eax, _kernel_buffer
    mov [dap_buffer_addr], ax

    # number of disk blocks to load
    mov word ptr [dap_blocks], 1

    # number of start block
    lea eax, _kernel_start_addr
    lea ebx, _start
    sub eax, ebx
    shr eax, 9 # divide by 512 (block size)
    mov [dap_start_lba], eax

    # destination address
    mov edi, 0x400000

    # block count
    lea ecx, [_kernel_end_addr]
    lea eax, [_kernel_start_addr]
    sub ecx, eax
    shr ecx, 9

load_next_kernel_block_from_disk:
    push esi
    lea esi, [loading_kernel_block_str]
    call println
    pop esi

    # load block from disk
    lea si, dap
    mov ah, 0x42
    int 0x13
    jc kernel_load_failed

    # copy block to 2MiB
    push ecx
    push esi
    mov ecx, 512 / 4
    movzx esi, word ptr [dap_buffer_addr]
    rep movsd [edi], [esi]
    pop esi
    pop ecx

    # next block
    mov eax, [dap_start_lba]
    add eax, 1
    mov [dap_start_lba], eax
    # add edi, 512

    sub ecx, 1
    jnz load_next_kernel_block_from_disk

check_cpu:
    call check_cpuid
    call check_long_mode

disable_irqs:
    mov al, 0xFF     # Out 0xFF to 0xA1 and 0x21 to disable all IRQs.
    out 0xA1, al
    out 0x21, al

    nop
    nop

    lidt zero_idt         # Load a zero length IDT so that any NMI causes a triple fault.

# enter long mode

set_up_page_tables:
    # zero out buffer for page tables
    lea edi, [_p4]
    mov ecx, 0x1000 / 4 * 3
    xor eax, eax
    rep stosd

    # p4
    lea eax, [_p3]
    or eax, (1 | 2)
    mov [_p4], eax
    # p3
    lea eax, [_p2]
    or eax, (1 | 2)
    mov [_p3], eax
    # p2
    mov eax, (1 | 2 | (1 << 7))
    mov ecx, 0
map_p2_table:
    mov [_p2 + ecx * 8], eax
    add eax, 0x200000
    add ecx, 1
    cmp ecx, 512
    jb map_p2_table

enable_paging:
    # load P4 to cr3 register (cpu uses this to access the P4 table)
    lea eax, [_p4]
    mov cr3, eax

    # enable PAE-flag in cr4 (Physical Address Extension)
    mov eax, cr4
    or eax, (1 << 5)
    mov cr4, eax

    # set the long mode bit in the EFER MSR (model specific register)
    mov ecx, 0xC0000080
    rdmsr
    or eax, (1 << 8)
    wrmsr

    # enable paging in the cr0 register
    mov eax, cr0
    or eax, ((1 << 31) | 1)
    mov cr0, eax

load_64bit_gdt:
    lgdt gdt_64_pointer                # Load GDT.Pointer defined below.

jump_to_long_mode:
    push 0x8
    lea eax, [long_mode]
    push eax
    retf # Load CS with 64 bit segment and flush the instruction cache

    jmp spin


check_cpuid:
    # Check if CPUID is supported by attempting to flip the ID bit (bit 21)
    # in the FLAGS register. If we can flip it, CPUID is available.

    # Copy FLAGS in to EAX via stack
    pushfd
    pop eax

    # Copy to ECX as well for comparing later on
    mov ecx, eax

    # Flip the ID bit
    xor eax, (1 << 21)

    # Copy EAX to FLAGS via the stack
    push eax
    popfd

    # Copy FLAGS back to EAX (with the flipped bit if CPUID is supported)
    pushfd
    pop eax

    # Restore FLAGS from the old version stored in ECX (i.e. flipping the
    # ID bit back if it was ever flipped).
    push ecx
    popfd

    # Compare EAX and ECX. If they are equal then that means the bit
    # wasn't flipped, and CPUID isn't supported.
    cmp eax, ecx
    je no_cpuid
    ret
no_cpuid:
    lea si, no_cpuid_str
    jmp error

check_long_mode:
    # test if extended processor info in available
    mov eax, 0x80000000    # implicit argument for cpuid
    cpuid                  # get highest supported argument
    cmp eax, 0x80000001    # it needs to be at least 0x80000001
    jb no_long_mode        # if it's less, the CPU is too old for long mode

    # use extended info to test if long mode is available
    mov eax, 0x80000001    # argument for extended processor info
    cpuid                  # returns various feature bits in ecx and edx
    test edx, (1 << 29)    # test if the LM-bit is set in the D-register
    jz no_long_mode        # If it's not set, there is no long mode
    ret
no_long_mode:
    lea si, no_long_mode_str
    jmp error


.align 4
zero_idt:
    .word 0
    .byte 0

gdt_64:
    .quad 0x0000000000000000          # Null Descriptor - should be present.
    .quad 0x00209A0000000000          # 64-bit code descriptor (exec/read).
    .quad 0x0000920000000000          # 64-bit data descriptor (read/write).

.align 4
    .word 0                              # Padding to make the "address of the GDT" field aligned on a 4-byte boundary

gdt_64_pointer:
    .word gdt_64_pointer - gdt_64 - 1    # 16-bit Size (Limit) of GDT.
    .long gdt_64                            # 32-bit Base Address of GDT. (CPU will zero extend to 64-bit)


.code64

long_mode:
    movabs rax, 0x00202f212f4b2f4f
    mov [0xb8000], rax

    # call load_elf with kernel start address and size as arguments
    lea rax, [_kernel_start_addr]
    lea rsi, [_kernel_end_addr]
    sub rsi, rax # calculate kernel size
    movabs rdi, 0x400000
    jmp load_elf
spin64:
    jmp spin64
