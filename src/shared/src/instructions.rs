global_asm!(include_str!("instructions.s"));

/// Performs a retf instruction, jumping to cs:eip
///
/// # Unsafety
/// We make no guarantees that the cs and eip are valid, nor that they contain executable code
#[inline(always)]
pub unsafe fn retf(cs: u16, eip: u32) {
	asm!("push {0:x}
		  push {1}
		  retf",
		  in(reg) cs, in(reg) eip);
}

/// Performs an iret instruction, jumping to cs:eip (as well as setting the stack to ss:esp and setting eflags)
///
/// # Unsafety
/// We make no guarantees that any of the parameters are valid
extern "C" {
	pub fn iret(ss: u32, esp: u32, cs: u32, eip: u32, eflags: u32);
}

/*
#[inline(always)]
pub unsafe fn iret(ss: u32, esp: u32, cs: u32, eip: u32, eflags: u32) {
	use crate::println;
	println!("ss - {} esp - {} cs - {} eip - {} eflags - {}", ss, esp, cs, eip, eflags);
	asm!("push {ss:e}
		  push {esp:e}
		  push {eflags:e}
		  push {cs:e}
		  push {eip:e}
		  iret",
	     ss = in(reg) ss, esp = in(reg) esp, cs = in(reg) cs, eip = in(reg) eip, eflags = in(reg) eflags
	);
}*/

/// Reads EFlags
#[inline]
pub fn read_eflags() -> u32 {
	let mut eflags: u32;

	unsafe {
    	asm!(
	    	"pushfd
		     pop {}", 
		     out(reg) eflags, options(nomem, preserves_flags)
    	)
    };

	eflags
}

/// Loads a new value into the task state register
///
/// # Unsafety
/// A bad value will cause undefined behaviour
#[inline(always)]
pub unsafe fn ltr(task_state: u16) {
	asm!("ltr {0:x}",
		 in(reg) task_state,
		 options(nostack)
	);
}

/// Halts the processor
#[inline(always)]
pub fn hlt() {
	unsafe {
    	asm!("hlt", options(nostack, nomem));
    }
}