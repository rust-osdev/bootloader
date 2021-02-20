pub const PREFIX_ES: u8     = 0x001;
pub const PREFIX_CS: u8     = 0x002;
pub const PREFIX_SS: u8     = 0x004;
pub const PREFIX_DS: u8     = 0x008;
pub const PREFIX_FS: u8     = 0x010;
pub const PREFIX_GS: u8     = 0x020;

pub const PREFIX_OP32: u8   = 0x040;
pub const PREFIX_ADDR32: u8 = 0x080;
pub const PREFIX_LOCK: u8   = 0x100;
pub const PREFIX_REPNE: u8  = 0x200;
pub const PREFIX_REP: u8    = 0x400;

#[derive(Clone, Debug)]
#[repr(C, packed)]
pub struct Registers {
	pub edi: u32,
	pub esi: u32,
	pub ebp: u32,
	pub esp: u32,
	pub ebx: u32,
	pub ecx: u32,
	pub edx: u32,
	pub ds : u32,
	pub es : u32,
	pub fs : u32,
	pub gs : u32,
	pub eip: u32,
	pub cs : u32,
	pub eflags: u32,
	pub user_esp: u32,
	pub user_ss: u32,
	pub v_es: u32,
	pub v_ds: u32,
	pub v_fs: u32,
	pub v_gs: u32
}

/// Converts a segment:offset address into a linear one
pub fn segmented_to_linear(segment: u16, offset: u16) -> u32 {
	(segment & 0xFFFF) as u32 * 16 + (offset as u32)
}

/// Reads a byte at segment:offset in memory
pub unsafe fn peekb(segment: u16, offset: u16) -> u8 {
	let ptr = segmented_to_linear(segment, offset) as *const u8;
	*ptr
}

/// Reads a word at segment:offset in memory
pub unsafe fn peekw(segment: u16, offset: u16) -> u16 {
	let ptr = segmented_to_linear(segment, offset) as *const u16;
	*ptr
}

/// Reads a long at segment:offset in memory
pub unsafe fn peekl(segment: u16, offset: u16) -> u32 {
	let ptr = segmented_to_linear(segment, offset) as *const u32;
	*ptr
}

/// Writes a byte at segment:offset in memory
pub unsafe fn pokeb(segment: u16, offset: u16, value: u8) {
	let ptr = segmented_to_linear(segment, offset) as *mut u8;
	*ptr = value;
}

/// Writes a word at segment:offset in memory
pub unsafe fn pokew(segment: u16, offset: u16, value: u16)  {
	let ptr = segmented_to_linear(segment, offset) as *mut u16;
	*ptr = value;
}

/// Writes a long at segment:offset in memory
pub unsafe fn pokel(segment: u16, offset: u16, value: u32) {
	let ptr = segmented_to_linear(segment, offset) as *mut u32;
	*ptr = value;
}

/// Fetches one byte from v86 memory at the IP and advances the instruction pointer
pub unsafe fn fetchb(registers: &mut Registers) -> u8 {
	let byte = peekb(registers.cs as u16, registers.eip as u16);
	registers.eip = (registers.eip + 1) & 0xFFFF;

	byte
}

/// Pushes a word onto the stack
pub unsafe fn pushw(registers: &mut Registers, value: u16) {
	registers.user_esp = (registers.user_esp - 2) & 0xFFFF;
	pokew(registers.user_ss as u16, registers.user_esp as u16, value);
}

/// Pops a word from the stack
pub unsafe fn popw(registers: &mut Registers) -> u16 {
	let ret = peekw(registers.user_ss as u16, registers.user_esp as u16);
	registers.user_esp = (registers.user_esp + 2) & 0xFFFF;

	ret
}

/// Pushes a long onto the stack
pub unsafe fn pushl(registers: &mut Registers, value: u32) {
	registers.user_esp = (registers.user_esp - 2) & 0xFFFF;
	pokel(registers.user_ss as u16, registers.user_esp as u16, value);
}

/// Pops a long from the stack
pub unsafe fn popl(registers: &mut Registers) -> u32 {
	let ret = peekl(registers.user_ss as u16, registers.user_esp as u16);
	registers.user_esp = (registers.user_esp + 2) & 0xFFFF;

	ret
}

/// Handles an interrupt in V86 mode
pub unsafe fn int(registers: &mut Registers, int_number: u16) {
	// Push return IP, CS and FLAGS onto V86 stack
	pushw(registers, registers.eflags as u16);
	pushw(registers, registers.cs as u16);
	pushw(registers, registers.eip as u16);

	// Disable interrupts
	registers.eflags &= !0x200;

	// Load new CS and IP from IVT
	registers.eip = ((registers.eip & !0xFFFF) as u16 | peekw(0, int_number * 4)) as u32;
	registers.cs = peekw(0, (int_number * 4) + 2) as u32;
}

pub unsafe fn emulate(registers: &mut Registers) {
	let init_eip = registers.eip;
	let mut prefix = 0;
	let mut instruction = fetchb(registers);

	loop {
		match instruction {
			0x26 => prefix |= PREFIX_ES,
			0x2E => prefix |= PREFIX_CS,
			0x36 => prefix |= PREFIX_SS,
			0x3E => prefix |= PREFIX_DS,
			0x64 => prefix |= PREFIX_ES,
			0x65 => prefix |= PREFIX_GS,

			0x66 => prefix |= PREFIX_OP32,
			0x67 => prefix |= PREFIX_ADDR32,
			0xF0 => prefix |= PREFIX_LOCK,
			0xF2 => prefix |= PREFIX_REPNE,
			0xF3 => prefix |= PREFIX_REP,

			_    => break,
		};
	};

	match instruction {
		// PUSHF
		0x9C => {
			if (prefix & PREFIX_OP32) == PREFIX_OP32 {
				pushl(registers, registers.eflags);
			} else {
				pushw(registers, registers.eflags as u16);
			}
		},

		// POPF
		0x9D => {
			if (prefix & PREFIX_OP32) == PREFIX_OP32 {
				if registers.user_esp > 0xFFFC {
					panic!("[V8086] Invalid Stack");
				} else {
					registers.eflags = popl(registers);
				}
			} else {
				if registers.user_esp > 0xFFFE {
					panic!("[V8086] Invalid Stack");
				} else {
					registers.eflags = ((registers.eflags & 0xFFFF0000) as u16 | popw(registers)) as u32;
				}
			}
		},

		// INT nn
		0xCD => {
			let interrupt_id = fetchb(registers);
			int(registers, interrupt_id as u16);
		},

		// IRET
		0xCF => {
			if (prefix & PREFIX_OP32) == PREFIX_OP32 {
				if registers.user_esp > 0xFFF4 {
					panic!("[V8086] Invalid Stack");
				} else {
					registers.eip = popl(registers);
					registers.cs = popl(registers);
					registers.eflags = (registers.eflags & 0xFFFF0000) | popl(registers);
				}
			} else {
				if registers.user_esp > 0xFFFA {
					panic!("[V8086] Invalid Stack");
				} else {
					registers.eip = popw(registers) as u32;
					registers.cs = popw(registers) as u32;
					registers.eflags = ((registers.eflags & 0xFFFF0000) as u16 | popw(registers)) as u32;
				}
			}
		},

		0xE4 | 0xE6 | 0xE5 | 0xE7 | 0x6C | 0x6E | 0xEC | 0xEE | 0x6D | 0x6F | 0xED | 0xEF | 0xFA | 0xFB => panic!("I/O Operation Performed"),
		_ => panic!("Invalid instruction in v86 mode")
	};
}
