ENTRY(_start)

SECTIONS {
    . = 0x500;
    _stack_start = .;
    . = 0x7c00;
    _stack_end = .;

    _bootloader_start = .;
    .boot :
    {
        *(.boot)
    }
    .text :
    {
        *(.text .text.*)
    }
    .data :
    {
        *(.rodata .rodata.*)
        *(.data .data.*)
        *(.got)
    }

    . = 0x7c00 + 510;
    .magic_number :
    {
        SHORT(0xaa55)       /* magic number for bootable disk */
    }

    _second_stage_start = .;
    .second_stage :
    {
        *(.second_stage)
    }
    . = ALIGN(512);
    _second_stage_end = .;
}
