ENTRY(_start)

SECTIONS {
    . = 0x7c00 + 512;

    .start : {
        *(.start)
    }
    .rodata : {
        *(.rodata .rodata.*)
    }
    .text : {
        *(.text .text.*)
    }

    . = ALIGN(512);

    _second_stage_end = .;
}
