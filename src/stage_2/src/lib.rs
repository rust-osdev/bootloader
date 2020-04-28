#![no_std]
#![feature(llvm_asm)]

use shared::console::println;
use shared::linker_symbol;
use v86::gdt::{GlobalDescriptorTable, Descriptor, TaskStateSegment};

use lazy_static::lazy_static;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();

        tss.privilege_stack_table[0].esp = linker_symbol!(_protected_mode_stack_end);

        tss
    };

    static ref GDT: GlobalDescriptorTable = {
        let mut gdt = GlobalDescriptorTable::new();
        gdt.add_entry(Descriptor::kernel_code_segment());
        gdt.add_entry(Descriptor::user_code_segment());
        gdt.add_entry(Descriptor::user_data_segment());

        gdt.add_entry(Descriptor::tss_segment(&TSS));

        gdt
    };
}

#[no_mangle]
pub fn second_stage() {
    println(b"Stage 2");

    println(b"Loading GDT");

    unsafe { GDT.load(); }

    println(b"GDT Loaded!");

    println(b"Switching to 32-bit");

    enable_a20();

    println(b"A20");

    unsafe {
        llvm_asm!("mov eax, cr0
                   or al, 1
                   mov cr0, eax

                   mov bx, 0x10
                   mov ds, bx
                   mov es, bx

                   jmp protected_mode" ::: "eax", "bx" : "intel", "volatile");
    }
}

static HELLO: &[u8] = b"Protected Mode!";

#[no_mangle]
extern "C" fn protected_mode() {
    let vga_buffer = 0xb8000 as *mut u8;

    for (i, &byte) in HELLO.iter().enumerate() {
        unsafe {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }

    loop {}
}

fn enable_a20() {
    unsafe {
        llvm_asm!("in al, 0x92
                   or al, 2
                   out 0x92, al" ::: "al" : "intel", "volatile");
    }
}