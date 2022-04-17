.section .boot, "awx"
.global _start
.code16

# This stage initializes the stack, enables the A20 line

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

    mov bx, 0x0f02         # attrib/char of smiley
    mov eax, 0xb8f00       # note 32 bit offset
    mov word ptr ds:[eax], bx

check_int13h_extensions:
    push 'y'    # error code
    mov ah, 0x41
    mov bx, 0x55aa
    # dl contains drive number
    int 0x13
    jc fail
    pop ax      # pop error code again

rust:
    # push arguments
    push dx     # disk number
    call first_stage

spin:
    hlt
    jmp spin

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
