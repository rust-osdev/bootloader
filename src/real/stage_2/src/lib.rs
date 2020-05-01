#![feature(llvm_asm)]
#![no_std]

// FIXME
#![allow(dead_code, unused_variables)]

use shared::linker_symbol;
use shared::println;
use shared::structures::gdt::{Descriptor, GlobalDescriptorTable, TaskStateSegment};

use lazy_static::lazy_static;

mod panic;

extern "C" {
    fn third_stage();
}

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();

        tss.privilege_stack_table[0].esp = linker_symbol!(_protected_mode_stack_end);

        tss
    };
    static ref GDT: GlobalDescriptorTable = {
        let mut gdt = GlobalDescriptorTable::new();
        gdt.add_entry(Descriptor::kernel_code_segment());
        gdt.add_entry(Descriptor::kernel_data_segment());
        gdt.add_entry(Descriptor::user_code_segment());
        gdt.add_entry(Descriptor::user_data_segment());

        gdt.add_entry(Descriptor::tss_segment(&TSS));

        gdt
    };
}

#[no_mangle]
pub fn second_stage() {
    println!("Stage 2");

    enter_protected_mode();

    loop {};
}

fn enter_protected_mode() {
    unsafe {
        GDT.load();
    }

    println!("Switching to Protected Mode");

    enable_a20();

    println!("A20 On");

    unsafe {
        llvm_asm!("cli

                   mov eax, cr0
                   or al, 1
                   mov cr0, eax

                   jmp third_stage" ::: "eax" : "intel", "volatile");
    }

    unreachable!();
}

fn enable_a20() {
    unsafe {
        llvm_asm!("in al, 0x92
                   or al, 2
                   out 0x92, al" ::: "al" : "intel", "volatile");
    }
}
