#![feature(global_asm, asm)]
#![no_std]

use shared::linker_symbol;
use shared::println;
use shared::structures::gdt::{Descriptor, GlobalDescriptorTable, TaskStateSegment};
use lazy_static::lazy_static;

mod v8086_code;
mod panic;

extern "C" {
    fn protected_mode_switch() -> !;
}

lazy_static! {
    static ref TSS: TaskStateSegment = {
        use core::mem::size_of;

        let mut tss = TaskStateSegment::new();

        tss.privilege_stack_table[0].esp = linker_symbol!(_protected_mode_stack_end);
        tss.privilege_stack_table[0].ss = 2 * 8; // Kernel data segment is 3rd segment (null, code, data)
        tss.iomap_base = size_of::<TaskStateSegment>() as u16; 

        tss
    };

    static ref GDT: GlobalDescriptorTable = {
        let mut gdt = GlobalDescriptorTable::new();

        // Set up kernel segments
        gdt.add_entry(Descriptor::kernel_code_segment());
        gdt.add_entry(Descriptor::kernel_data_segment());

        // Set up user segments
        gdt.add_entry(Descriptor::user_code_segment());
        gdt.add_entry(Descriptor::user_data_segment());

        // Set up the TSS
        gdt.add_entry(Descriptor::tss_segment(&*TSS));
        
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

        println!("[Bootloader] [16] Loaded GDT");
        
        protected_mode_switch();
    }
}

fn enable_a20() {
    unsafe {
        asm!("in {0}, 0x92
              or {0}, 2
              out 0x92, {0}",
            out(reg) _,
            options(nostack)
        );
    }
}
