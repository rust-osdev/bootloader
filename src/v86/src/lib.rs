#![feature(abi_x86_interrupt)]
#![feature(const_fn)]
#![feature(llvm_asm)]
#![no_std]

pub mod idt;
pub mod monitor;

pub struct V86 {}

impl V86 {
    fn gpf_handler(frame: idt::InterruptStackFrame) {
        let instruction_pointer = ((frame.cs << 4) + frame.eip) as *const _ as u16;
        let instructions = slice::from_raw_parts_mut(instruction_pointer, 2);

        let ivt_pointer = 0 as *const _ as u16;
        let ivt = slice::from_raw_parts_mut(ivt_pointer, 1024);

        let stack_pointer = ((frame.ss << 4) + frame.esp) as u16;
        let stack = slice::from_raw_parts_mut(stack_pointer, 16);

        match instructions[0] {
            0xcd => match instructions[1] {
                0xff => {
                    match frame.eax {
                        0x0 => {}, // Terminate V86
                        0x1 => {}, // Copy from 1MB buffer
                        _ => panic!("Invalid V86 Monitor Function")
                    }
                },
                _ => {
                    stack -= 3;
                    frame.esp = ((frame.esp) & 0xffff) - 6) & 0xffff;
                    
                    stack[0] = (frame.eip + 2) as u16;
                    stack[1] = frame.cs;
                    stack[2] = frame.eflags as u16;

                    if current.v86_if {
                        stack[2] |= EFLAG_IF;
                    } else {
                        stack[2] &= ~EFLAG_IF;
                    }

                    current.v86_if = false;
                    frame.cs = ivt[instruction_pointer[1] * 2 + 1];
                    frame.eip = ivt[instruction_pointer[1] * 2];
                }
            },
            _ => {}
        }
    }
}