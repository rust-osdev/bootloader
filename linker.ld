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
    }

    /* We write the magic number in the linker script to cause a link-time error if the bootloader overflows the 512 byte limit */
    . = 0x7c00 + 510;
    .magic :
    {
        SHORT(0xaa55)
    }

    .rest_of_bootloader :
    {
        _rest_of_bootloader_start = .;

        *(.text .text.*)
        *(.data .data.*)
        *(.rodata .rodata.*)
        *(.bss .bss.*)

        . = ALIGN(512);
    
        _rest_of_bootloader_end = .;
    }

    _bootloader_end = .;

    _protected_mode_stack_start = .;
    . += 0x1000;
    _protected_mode_stack_end = .;
    
    /DISCARD/ :
    {
        *(.eh_frame)
    }
}