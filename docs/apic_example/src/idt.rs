use crate::apic;
use crate::gdt::DOUBLE_FAULT_IST_INDEX;
use lazy_static::lazy_static;
use log::info;
use x86_64::instructions::hlt;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

pub const PIC_1_OFFSET: u8 = 0x20;

lazy_static! {
    pub static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        idt.breakpoint.set_handler_fn(handle_breakpoint);
        idt.page_fault.set_handler_fn(handle_page_fault);

        idt[InterruptIndex::Timer as u8].set_handler_fn(handle_timer);
        idt[InterruptIndex::Keyboard as u8].set_handler_fn(handle_keyboard);

        unsafe {
            idt.double_fault
                .set_handler_fn(handle_double_fault)
                .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        }

        idt
    };
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

pub extern "x86-interrupt" fn handle_timer(_stack_frame: InterruptStackFrame) {
    // Handle logic

    apic::end_interrupt();
}

pub extern "x86-interrupt" fn handle_breakpoint(stack_frame: InterruptStackFrame) {
    info!("Breakpoint hit:\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn handle_double_fault(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    info!("\nDouble fault:\n{:#?}", stack_frame);

    loop {
        hlt()
    }
}

pub extern "x86-interrupt" fn handle_page_fault(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    info!("Exception        : Page Fault");
    info!("Accessed address : {:?}", Cr2::read());
    info!("ErrorCode        : {:?}", error_code);
    info!("{:#?}", stack_frame);

    loop {
        hlt()
    }
}

pub extern "x86-interrupt" fn handle_keyboard(_stack_frame: InterruptStackFrame) {
    // Handle logic

    apic::end_interrupt();
}


