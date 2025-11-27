ENTRY(_start)

SECTIONS {
    . = 0x7c00 + 512;

    .start : {
        *(.start)
    }
    .bss : {
        *(.bss .bss.*)
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
    .eh_frame : {
        *(.eh_frame .eh_frame.*)
    }
    .eh_frame_hdr : {
        *(.eh_frame_hdr .eh_frame_hdr.*)
    }

    . = ALIGN(512);

    _second_stage_end = .;

    . = 0x0007FFFF - 2;
    .end_marker :
    {
        SHORT(0xdead)
    }
}
