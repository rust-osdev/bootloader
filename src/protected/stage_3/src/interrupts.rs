use shared::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::println;
use lazy_static::lazy_static;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        //idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);

        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);

        idt
    };
}


pub fn init_idt() {
	// Seems like we have to manually initialize it first for some reason, otherwise it crashes
	::lazy_static::initialize(&IDT);
	IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: &mut InterruptStackFrame)
{
    println!("[Bootloader] [IDT] Breakpoint Hit @ {}:{}", stack_frame.cs, stack_frame.eip);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame, _error_code: u32) -> !
{
    panic!("[Bootloader] [IDT] Double Fault!");
}

extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: &mut InterruptStackFrame, error_code: u32)
{
    println!("[Bootloader] [IDT] #NP {} ({})", stack_frame.eip, error_code);
    loop {};
}

extern "x86-interrupt" fn general_protection_fault_handler(
	stack_frame: &mut InterruptStackFrame, error_code: u32)
{
    println!("{:?}", stack_frame);
    // VM Bit
    if stack_frame.eflags & (1 << 17) == (1 << 17) {
//        loop {};
//        v8086_handler(stack_frame);
        println!("VM Bit Set");
    }
    println!("[Bootloader] [IDT] GPF {} ({})", stack_frame.eip, error_code);
    loop {};
}