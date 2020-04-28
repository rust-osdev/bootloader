#![feature(abi_x86_interrupt)]
#![feature(const_fn)]
#![feature(llvm_asm)]
#![no_std]

pub mod idt;

const EFLAG_IF: u16 = 0x00000200;
const EFLAG_VM: u16 = 0x00020000;

pub struct V86 {}

impl V86 {
    extern "x86-interrupt" fn gpf_handler(frame: &mut idt::InterruptStackFrame, error_code: u64) {
        // Calculate the V86 Instruction Pointer and create a slice of it
        let instruction_pointer = ((frame.cs << 4) + frame.eip) as *const _ as u16;
        let instructions = slice::from_raw_parts_mut(instruction_pointer, 2);

        // Calculate the V86 IVT pointer and create a slice of it
        let ivt_pointer = 0 as *const _ as u16;
        let ivt = slice::from_raw_parts_mut(ivt_pointer, 1024);

        // Calculate the V86 stack pointer and create a slice of it
        let stack_pointer = ((frame.ss << 4) + frame.esp) as *const _ as u16;
        let stack = slice::from_raw_parts_mut(stack_pointer, 16);

        // Match the first byte of the instruction
        match instructions[0] {
            // int <interrupt> 
            0xcd => match instructions[1] {
                // 0xFF (255) is our V86 monitor interrupt
                0xff => {
                    // EAX stores the "function" to use. Similar to BIOS interrupts
                    match frame.eax {
                        // Terminate V86
                        0x0 => unimplemented!(),

                        // Copy data into protected mode address space
                        0x1 => {
                            // EBX - Real mode address
                            // ECX - Protected mode address
                            // EDX - Amount of bytes to copy
                            let source_pointer = frame.ebx as *const _ as u32;
                            let destination_pointer = frame.ecx as *const _ as u32;

                            let size = frame.edx;

                            let source = slice::from_raw_parts(source_pointer, size);
                            let mut destination = slice::from_raw_parts_mut(destination_pointer, size);

                            destination.clone_from_slice(source);
                        },
                        _ => panic!("Invalid V86 Monitor Function")
                    };
                },
                _ => {
                    // All other interrupt vectors are processed by the real mode handlers
                    stack -= 3;
                    frame.esp = ((frame.esp) & 0xffff) - 6) & 0xffff;

                    // Store the next instructions EIP and code segment onto the stack                    
                    stack[0] = (frame.eip + 2) as u16;
                    stack[1] = frame.cs;

                    // Store the EFlags onto the stack
                    stack[2] = frame.eflags as u16;

                    // Set the CS and EIP to the real mode interrupt handler
                    frame.cs = ivt[instruction_pointer[1] * 2 + 1];
                    frame.eip = ivt[instruction_pointer[1] * 2];
                }
            },
            // iret
            0xcf => {
                frame.eip = stack[0];
                frame.cs = stack[1];
                frame.eflags = EFLAG_IF | EFLAG_VM | stack[2];

                frame.esp = ((frame.esp & 0xffff) + 6) & 0xffff;
            },
            _ => panic!("Unhandled GPF")
        };
    }
}