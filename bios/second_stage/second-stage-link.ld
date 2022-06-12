ENTRY(_start)

SECTIONS {
    . = 0x7c00 + 512;

    .start : {
        *(.start)
    }
    .text : ALIGN(512) {
        *(.text .text.*)
    }
    .rodata : ALIGN(512) {
        *(.rodata .rodata.*)
    }
    .bss : ALIGN(512) {
        *(.bss .bss.*)
    }
    .data : ALIGN(512) {
        *(.data .data.*)
    }

    . = ALIGN(512);

    _second_stage_end = .;
}
