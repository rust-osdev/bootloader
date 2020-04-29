#![feature(llvm_asm)]
#![no_std]
#![no_main]

// FIXME
#![allow(dead_code, unused_variables)]

use shared::println;
use shared::linker_symbol;
use v86::gdt::{GlobalDescriptorTable, Descriptor, TaskStateSegment};

use lazy_static::lazy_static;

mod panic;

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
    println!("Stage 2");

    loop {}
}

fn enter_protected_mode() {
    println!("Loading GDT");

    unsafe { GDT.load(); }

    println!("GDT Loaded!");

    println!("Switching to 32-bit");

    enable_a20();

    println!("A20");

    loop {};

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

#[no_mangle]
extern "C" fn protected_mode() {
    println!("Protected Mode!");

    loop {} 
}

fn enable_a20() {
    unsafe {
        llvm_asm!("in al, 0x92
                   or al, 2
                   out 0x92, al" ::: "al" : "intel", "volatile");
    }
}