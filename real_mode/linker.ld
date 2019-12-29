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

    .real_mode_text :
    {
        *real_mode*(.text .text.*)
    }
    .real_mode_data :
    {
        *real_mode*(.rodata .rodata.*)
        *real_mode*(.data .data.*)
        *real_mode*(.got .got.*)
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
