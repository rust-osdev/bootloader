ENTRY(_start)

SECTIONS {
    . = 0x500;
    _stack_start = .;
    . = 0x7c00;
    _stack_end = .;

    _bootloader_start = .;
    .boot :
    {
        *first_stage*(.boot)
    }
    .first_stage_text :
    {
        *first_stage*(.text .text.*)
    }
    .first_stage_data :
    {
        *first_stage*(.rodata .rodata.*)
        *first_stage*(.data .data.*)
        *first_stage*(.got .got.*)
    }

    . = 0x7c00 + 510;
    .magic_number :
    {
        SHORT(0xaa55)       /* magic number for bootable disk */
    }

    _second_stage_start = .;

    .second_stage_text :
    {
        *second_stage*(.text .text.*)
    }
    .second_stage_data :
    {
        *second_stage*(.rodata .rodata.*)
        *second_stage*(.data .data.*)
        *second_stage*(.got .got.*)
    }

    .text :
    {
        *(.text .text.*)
    }

    .data :
    {
        *(.rodata .rodata.*)
        *(.data .data.*)
        *(.got .got.*)
    }

    .asm :
    {
        *(.second_stage)
    }
    . = ALIGN(512);
    _second_stage_end = .;
}
