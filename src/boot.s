.section .boot, "awx"
.global _start
.intel_syntax noprefix
.code16

_start:
    # zero segment registers
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    # TODO explain
    cld


    # initialize stack
    mov sp, 0x7c00

    lea si, boot_start_str
    call println

enable_a20:
    # enable A20-Line via IO-Port 92, might not work on all motherboards
    in al, 0x92
    or al, 2
    out 0x92, al

enter_protected_mode:
    cli
    push ds
    push es

    lgdt [gdtinfo]

    mov eax, cr0
    or al, 1    # set protected mode bit
    mov cr0, eax

    jmp protected_mode                # tell 386/486 to not crash

protected_mode:
    mov bx, 0x8
    mov ds, bx # set data segment
    mov es, bx # set extra segment

    and al, 0xfe    # clear protected mode bit
    mov cr0, eax

unreal_mode:
    pop es # get back old extra segment
    pop ds # get back old data segment
    sti

    # back to real mode, but internal data segment register is still loaded
    # with gdt segment -> we can access the full 4GiB of memory

    mov bx, 0x0f01         # attrib/char of smiley
    mov eax, 0xb8f00       # note 32 bit offset
    mov word ptr ds:[eax], bx

check_int13h_extensions:
    mov ah, 0x41
    mov bx, 0x55aa
    # dl contains drive number
    int 0x13
    jc no_int13h_extensions

load_second_stage_from_disk:
    lea eax, _second_stage_start_addr

    # start of memory buffer
    mov [dap_buffer_addr], ax

    # number of disk blocks to load
    lea ebx, _second_stage_end_addr
    sub ebx, eax # second stage end - second stage start
    shr ebx, 9 # divide by 512 (block size)
    mov [dap_blocks], bx

    # number of start block
    lea ebx, _start
    sub eax, ebx
    shr eax, 9 # divide by 512 (block size)
    mov [dap_start_lba], eax

    lea si, dap
    mov ah, 0x42
    int 0x13
    jc second_stage_load_failed

jump_to_second_stage:
    lea eax, second_stage
    jmp eax

spin:
    jmp spin

# print a string and a newline
# IN
#   si: points at zero-terminated String
# CLOBBER
#   ax
println:
    call print
    mov al, 13 # \r
    call print_char
    mov al, 10 # \n
    jmp print_char

# print a string
# IN
#   si: points at zero-terminated String
# CLOBBER
#   ax
print:
    cld
print_loop:
    lodsb
    test al, al
    jz print_done
    call print_char
    jmp print_loop
print_done:
    ret

# print a character
# IN
#   al: character to print
# CLOBBER
#   ah
print_char:
    mov ah, 0x0e
    int 0x10
    ret

# print a number in hex
# IN
#   bx: the number
# CLOBBER
#   al, cx
print_hex:
    mov cx, 4
.lp:
    mov al, bh
    shr al, 4

    cmp al, 0xA
    jb .below_0xA

    add al, 'A' - 0xA - '0'
.below_0xA:
    add al, '0'

    call print_char

    shl bx, 4
    loop .lp

    ret

error:
    call println
    jmp spin

no_int13h_extensions:
    lea si, no_int13h_extensions_str
    jmp error

second_stage_load_failed:
    lea si, second_stage_load_failed_str
    jmp error

kernel_load_failed:
    lea si, kernel_load_failed_str
    jmp error

boot_start_str: .asciz "Booting (first stage)..."
second_stage_start_str: .asciz "Booting (second stage)..."
error_str: .asciz "Error: "
no_cpuid_str: .asciz "No CPUID support"
no_int13h_extensions_str: .asciz "No support for int13h extensions"
second_stage_load_failed_str: .asciz "Failed to load second stage of bootloader"
kernel_load_failed_str: .asciz "Failed to load kernel"
no_long_mode_str: .asciz "No long mode support"

gdtinfo:
   .word gdt_end - gdt - 1  # last byte in table
   .word gdt                # start of table

gdt:
    # entry 0 is always unused
    .quad 0
flatdesc:
    .byte 0xff
    .byte 0xff
    .byte 0
    .byte 0
    .byte 0
    .byte 0b10010010
    .byte 0b11001111
    .byte 0
gdt_end:

dap: # disk access packet
    .byte 0x10 # size of dap
    .byte 0 # unused
dap_blocks:
    .word 0 # number of sectors
dap_buffer_addr:
    .word 0 # offset to memory buffer
dap_buffer_seg:
    .word 0 # segment of memory buffer
dap_start_lba:
    .quad 0 # start logical block address

.org 510
.word 0xaa55 # magic number for bootable disk
