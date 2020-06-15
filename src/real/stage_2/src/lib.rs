#![feature(global_asm, llvm_asm)]
#![no_std]

use shared::linker_symbol;
use shared::println;
use shared::structures::gdt::{Descriptor, GlobalDescriptorTable, TaskStateSegment};
use lazy_static::lazy_static;

mod panic;

extern "C" {
    fn protected_mode_switch() -> !;
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

        gdt.add_entry(Descriptor::tss_segment(&TSS));

        gdt
    };
}

global_asm!(include_str!("protected_mode.s"));

#[no_mangle]
pub fn second_stage() -> ! {
    println!("[Bootloader] [16] Stage 2");

    enable_a20();

    unsafe {
        GDT.load();
        
        protected_mode_switch();
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
