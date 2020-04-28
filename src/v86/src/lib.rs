#![no_std]
pub mod monitor;

pub struct V86 {}

impl V86 {
    fn gpf_handler(frame: InterruptStackFrame) {
        let instruction_pointer = ((frame.cs << 4) + frame.eip) as u16;
        let ivt = 0u16;
        let stack = ((frame.ss << 4) + frame.esp) as u16;
        let stack32 = stack as u32;

        match instruction_pointer[0] {
            0xcd => match instruction_pointer[1] {
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