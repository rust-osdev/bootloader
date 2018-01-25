ENTRY(_start)

SECTIONS {
    . = 0x500;
    /* buffer for loading the kernel */
    _kernel_buffer = .;
    . += 512;
    /* page tables */
    . = ALIGN(0x1000);
    _p4 = .;
    . += 0x1000;
    _p3 = .;
    . += 0x1000;
    _p2 = .;
    . += 0x1000;
    _memory_map = .;
    . += 0x1000;

    /* bootloader */
    . = 0x7c00;
    .bootloader :
    {
        /* first stage */
        *(.boot)

        /* second stage */
        _second_stage_start_addr = .;
        *(.second_stage)
        *(.context_switch)
        *(.text .text.*)
        *(.rodata .rodata.*)
        *(.data .data.*)
        . = ALIGN(512);
        _second_stage_end_addr = .;
    }

    _kernel_inferface_block_start = .;
    _kib_kernel_size = .;
    . += 512; /* kernel interface block */
    _kernel_inferface_block_end = .;

    _kernel_start_addr = .;
}
