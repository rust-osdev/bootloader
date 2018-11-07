.section .boot, "awx"
.global first_stage
.intel_syntax noprefix
.code16

# This stage initializes the stack, enables the A20 line, loads the rest of
# the bootloader from disk, and jumps to it.

first_stage:
    # zero segment registers
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    # clear the direction flag (e.g. go forward in memory when using
    # instructions like lodsb)
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

check_int13h_extensions:
    mov ah, 0x41
    mov bx, 0x55aa
    # dl contains drive number
    int 0x13
    jc no_int13h_extensions

load_bootloader_rest_from_disk:
    lea eax, _bootloader_rest_start_addr

    # start of memory buffer
    mov [dap_buffer_addr], ax

    # number of disk blocks to load
    lea ebx, _kernel_info_block_end
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
    jc bootloader_rest_load_failed

jump_to_second_stage:
    jmp second_stage
spin_1st_stage:
    jmp spin_1st_stage

# print a string and a newline
# IN
#   esi: points at zero-terminated String
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
#   esi: points at zero-terminated String
# CLOBBER
#   ax
print:
    cld
print_loop:
    # note: if direction flag is set (via std)
    # this will DECREMENT the ptr, effectively
    # reading/printing in reverse.
    lodsb al, BYTE PTR [esi]
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

error:
    call println
error_spin:
    jmp spin

no_int13h_extensions:
    lea si, no_int13h_extensions_str
    jmp error

bootloader_rest_load_failed:
    lea si, bootloader_rest_load_failed_str
    jmp error

boot_start_str: .asciz "Booting (first stage)..."
no_int13h_extensions_str: .asciz "No support for int13h extensions"
bootloader_rest_load_failed_str: .asciz "Failed to load rest of bootloader"

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
