ENTRY(_start)

SECTIONS {
    # must match STAGE_4_DST address in bios/stage-2/src/main.rs
    . = 0x00130000;

    .start : {
        *(.start)
    }
    .text : {
        *(.text .text.*)
    }
    .rodata : {
        *(.rodata .rodata.*)
    }
    .data : {
        *(.data .data.*)
    }
    .bss : {
        *(.bss .bss.*)
    }
}
