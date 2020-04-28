ENTRY(_start)

SECTIONS {
    . = 0x500;
    _stack_start = .;
    . = 0x7c00;
    _stack_end = .;

    .bootsector :
    {
        _bootloader_start = .;

        *(.bootstrap)
        *bootsector*

        *shared*
    }

    /* We write the magic number in the linker script to cause a link-time error if the bootloader overflows the 512 byte limit */
    . = 0x7c00 + 510;
    .magic :
    {
        SHORT(0xaa55)
    }

    _rest_of_bootloader_start = .;

    .stage_2 :
    {

        _second_stage_start = .;

        *stage_2*

        _second_stage_end = .;
    }

    . = ALIGN(512);

    _rest_of_bootloader_end = .;
    _bootloader_end = .;
    
    /DISCARD/ :
    {
        *(.eh_frame)
    }
}