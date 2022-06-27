ENTRY(_start)

SECTIONS {
    . = 0x7c00 + 512;

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
        SHORT(0xdead)       /* magic number for bootable disk */
    }
    . = 0x7c00 + 512;
}
