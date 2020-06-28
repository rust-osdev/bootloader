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