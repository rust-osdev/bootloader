ENTRY(_start)

SECTIONS {
    . = 0x500;
    _stack_start = .;
    . = 0x7c00;
    _stack_end = .;

    .bootloader :
    {
        *(.boot-first-stage)
        *(.text .text.*)
        *(.rodata .rodata.*)
        *(.data .data.*)
        *(.got)
        . = 0x7c00 + 510;
        SHORT(0xaa55)       /* magic number for bootable disk */
    }
    _bootloader_end = .;
}
