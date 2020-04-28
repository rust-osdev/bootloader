#![feature(abi_x86_interrupt)]
#![feature(const_fn)]
#![feature(llvm_asm)]
#![no_std]

use core::slice;

pub mod gdt;
pub mod idt;

const EFLAG_IF: u32 = 0x00000200;
const EFLAG_VM: u32 = 0x00020000;
/*
pub struct V86 {}

impl V86 {
    unsafe extern "x86-interrupt" fn gpf_handler(frame: &mut idt::InterruptStackFrame, error_code: u64) {
        // Calculate the V86 Instruction Pointer and create a slice of it
        let instruction_pointer = ((frame.cs << 4) + frame.eip) as *const u16;
        let instructions = slice::from_raw_parts(instruction_pointer, 2);

        // Calculate the V86 IVT pointer and create a slice of it
        let ivt_pointer = 0 as *const u16;
        let ivt = slice::from_raw_parts(ivt_pointer, 1024);

        // Calculate the V86 stack pointer and create a slice of it
        let mut stack_pointer = ((frame.ss << 4) + frame.esp) as *mut u16;

        let mut stack = slice::from_raw_parts_mut(stack_pointer, 16);
        let mut stack32 = slice::from_raw_parts_mut(stack_pointer as *mut u32, 8);

        // Match the first byte of the instruction
        match instructions[0] {
            // int <interrupt> 
            0xcd => match instructions[1] {
                // 0xFF (255) is our V86 monitor interrupt
                0xff => {
                    // Function should be pushed onto stack first
                    let function = stack[3];

                    match function {
                        // Terminate V86
                        0x0 => unimplemented!(),

                        // push 0x2000   - stack[6] Size
                        // push 0x1000   - stack[5] Real Mode Buffer
                        // push 0x100000 - stack[4] Protected Mode Buffer
                        // push 0x1      - stack[3] Function

                        // Copy data into protected mode address space
                        0x1 => {
                            // We read buffer addresses and size from 
                            let destination_pointer = stack[4] as *mut u32;
                            let source_pointer = stack[5] as *const u32;
                            let size = stack[6];

                            let source = slice::from_raw_parts(source_pointer, size);
                            let mut destination = slice::from_raw_parts_mut(destination_pointer, size);

                            destination.clone_from_slice(source);
                        },
                        _ => panic!("Invalid V86 Monitor Function")
                    };
                },
                _ => {
                    // All other interrupt vectors are processed by the real mode handlers
                    stack_pointer = stack_pointer.offset(3);
                    frame.esp = ((frame.esp & 0xffff) - 6) & 0xffff;

                    // Store the next instructions EIP and code segment onto the stack                    
                    stack[0] = (frame.eip + 2) as usize;
                    stack32[1] = frame.cs;

                    // Store the EFlags onto the stack
                    stack[2] = frame.eflags as usize;

                    // Set the CS and EIP to the real mode interrupt handler
                    frame.cs = ivt[(instructions[1] * 2 + 1) as usize] as u32;
                    frame.eip = ivt[(instructions[1] * 2) as usize] as u32;
                }
            },
            // iret
            0xcf => {
                frame.eip = stack32[0];
                frame.cs = stack32[1];
                frame.eflags = EFLAG_IF | EFLAG_VM | stack32[2];

                frame.esp = ((frame.esp & 0xffff) + 6) & 0xffff;
            },
            _ => panic!("Unhandled GPF")
        };
    }
}*/