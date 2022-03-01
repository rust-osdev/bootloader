.section .boot-first-stage, "awx"
.global _start
.code16

# This stage initializes the stack, enables the A20 line, loads the rest of
# the bootloader from disk, and jumps to stage_2.

_start:
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

    mov si, offset boot_start_str
    call real_mode_println

enable_a20:
    # enable A20-Line via IO-Port 92, might not work on all motherboards
    in al, 0x92
    test al, 2
    jnz enable_a20_after
    or al, 2
    and al, 0xFE
    out 0x92, al
enable_a20_after:


enter_protected_mode:
    # clear interrupts
    cli
    push ds
    push es

    lgdt [gdt32info]

    mov eax, cr0
    or al, 1    # set protected mode bit
    mov cr0, eax

    jmp protected_mode                # tell 386/486 to not crash

protected_mode:
    mov bx, 0x10
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

load_rest_of_bootloader_from_disk:
    mov eax, offset _rest_of_bootloader_start_addr

    mov ecx, 0

load_from_disk:
    lea eax, _rest_of_bootloader_start_addr
    add eax, ecx # add offset

    # dap buffer segment
    mov ebx, eax
    shr ebx, 4 # divide by 16
    mov [dap_buffer_seg], bx

    # buffer offset
    shl ebx, 4 # multiply by 16
    sub eax, ebx
    mov [dap_buffer_addr], ax

    mov eax, offset _rest_of_bootloader_start_addr
    add eax, ecx # add offset

    # number of disk blocks to load
    mov ebx, offset _rest_of_bootloader_end_addr
    sub ebx, eax # end - start
    jz load_from_disk_done
    shr ebx, 9 # divide by 512 (block size)
    cmp ebx, 127
    jle .continue_loading_from_disk
    mov ebx, 127
.continue_loading_from_disk:
    mov [dap_blocks], bx
    # increase offset
    shl ebx, 9
    add ecx, ebx

    # number of start block
    mov ebx, offset _start
    sub eax, ebx
    shr eax, 9 # divide by 512 (block size)
    mov [dap_start_lba], eax

    mov si, offset dap
    mov ah, 0x42
    int 0x13
    jc rest_of_bootloader_load_failed

    jmp load_from_disk

load_from_disk_done:
    # reset segment to 0
    mov word ptr [dap_buffer_seg], 0

jump_to_second_stage:
    mov eax, offset stage_2
    jmp eax

spin:
    jmp spin

# print a string and a newline
# IN
#   si: points at zero-terminated String
# CLOBBER
#   ax
real_mode_println:
    call real_mode_print
    mov al, 13 # \r
    call real_mode_print_char
    mov al, 10 # \n
    jmp real_mode_print_char

# print a string
# IN
#   si: points at zero-terminated String
# CLOBBER
#   ax
real_mode_print:
    cld
real_mode_print_loop:
    # note: if direction flag is set (via std)
    # this will DECREMENT the ptr, effectively
    # reading/printing in reverse.
    lodsb al, BYTE PTR [si]
    test al, al
    jz real_mode_print_done
    call real_mode_print_char
    jmp real_mode_print_loop
real_mode_print_done:
    ret

# print a character
# IN
#   al: character to print
# CLOBBER
#   ah
real_mode_print_char:
    mov ah, 0x0e
    int 0x10
    ret

# print a number in hex
# IN
#   bx: the number
# CLOBBER
#   al, cx
real_mode_print_hex:
    mov cx, 4
.lp:
    mov al, bh
    shr al, 4

    cmp al, 0xA
    jb .below_0xA

    add al, 'A' - 0xA - '0'
.below_0xA:
    add al, '0'

    call real_mode_print_char

    shl bx, 4
    loop .lp

    ret

real_mode_error:
    call real_mode_println
    jmp spin

no_int13h_extensions:
    mov si, offset no_int13h_extensions_str
    jmp real_mode_error

rest_of_bootloader_load_failed:
    mov si, offset rest_of_bootloader_load_failed_str
    jmp real_mode_error

boot_start_str: .asciz "Booting (first stage)..."
error_str: .asciz "Error: "
no_int13h_extensions_str: .asciz "No support for int13h extensions"
rest_of_bootloader_load_failed_str: .asciz "Failed to load rest of bootloader"

gdt32info:
   .word gdt32_end - gdt32 - 1  # last byte in table
   .word gdt32                  # start of table

gdt32:
    # entry 0 is always unused
    .quad 0
codedesc:
    .byte 0xff
    .byte 0xff
    .byte 0
    .byte 0
    .byte 0
    .byte 0x9a
    .byte 0xcf
    .byte 0
datadesc:
    .byte 0xff
    .byte 0xff
    .byte 0
    .byte 0
    .byte 0
    .byte 0x92
    .byte 0xcf
    .byte 0
gdt32_end:

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
