use shared::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::println;
use lazy_static::lazy_static;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        idt.divide_error.set_handler_fn(divide_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);

        idt.double_fault.set_handler_fn(double_fault_handler);

        idt
    };
}


pub fn init_idt() {
	// Seems like we have to manually initialize it first for some reason, otherwise it crashes
	::lazy_static::initialize(&IDT);
	IDT.load();
}

extern "x86-interrupt" fn divide_handler(
    stack_frame: &mut InterruptStackFrame)
{
    println!("[Bootloader] [IDT] Divide Exception");
}

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: &mut InterruptStackFrame)
{
    println!("[Bootloader] [IDT] Breakpoint Hit");
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame, _error_code: u32) -> !
{
    panic!("[Bootloader] [IDT] Double Fault!");
}