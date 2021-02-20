//! Virtual 8086 Mode monitor and entry code
use core::mem;

#[macro_use]
mod macros;

const PFX_ES: u16 = 0x001;
const PFX_CS: u16 = 0x002;
const PFX_SS: u16 = 0x004;
const PFX_DS: u16 = 0x008;
const PFX_FS: u16 = 0x010;
const PFX_GS: u16 = 0x020;

const PFX_OP32: u16 = 0x040;
const PFX_ADDR32: u16 = 0x080;
const PFX_LOCK: u16 = 0x100;
const PFX_REPNE: u16 = 0x200;
const PFX_REP: u16 = 0x400;

// V8086 Entry Code
global_asm!(include_str!("enter_v8086.s"));

extern "C" {
    fn _enter_v8086(eip: u32);
}

/// Trait for integers usable as peek / poke / pop / push / etc.
trait IntegerValue: Copy {}
impl IntegerValue for u8 {}
impl IntegerValue for u16 {}
impl IntegerValue for u32 {}

/// Stack info
#[derive(Debug)]
pub struct Stack {
	segment: u32,
	offset: u32
}

impl Stack {
	pub fn new(segment: u32, offset: u32) -> Stack {
		Stack {
			segment: segment,
			offset: offset
		}
	}
}

/// Registers
#[derive(Debug)]
#[repr(C)]
#[repr(packed)]
pub struct Registers {
	edi: u32,
	esi: u32,
	ebp: u32,
	esp: u32,
	ebx: u32,
	edx: u32,
	ecx: u32,
	eax: u32,

	ds: u32,
	es: u32,
	fs: u32,
	gs: u32,

	eip: u32,
	cs: u32,
	eflags: u32,
	user_esp: u32,
	user_ss: u32,

	v_es: u32,
	v_ds: u32,
	v_fs: u32,
	v_gs: u32
}

impl Registers {
	/// Creates a new set of registers (all zero)
	pub fn new() -> Registers {
		Registers {
			edi: 0,
			esi: 0,
			ebp: 0,
			esp: 0,
			ebx: 0,
			edx: 0,
			ecx: 0,
			eax: 0,

			ds: 0,
			es: 0,
			fs: 0,
			gs: 0,

			eip: 0,
			cs: 0,
			eflags: 0,
			user_esp: 0,
			user_ss: 0,

			v_es: 0,
			v_ds: 0,
			v_fs: 0,
			v_gs: 0,
		}
	}

	/// Saves the current state of CPU registers into ourselves
	pub unsafe extern "C" fn save(&mut self) {
		// TODO

	}
}

#[repr(C)]
#[repr(packed)]
struct V86Registers {
	pub gs: u32,
	pub fs: u32,
	pub ds: u32,
	pub es: u32,

	pub ss: u32,
	pub esp: u32,

	pub eflags: u32,

	pub cs: u32,
	pub eip: u32
}

/// The V8086 Monitor itself
#[derive(Debug)]
pub struct Monitor {
	pub stack: Stack,
	pub registers: Registers
}

impl Monitor {
	/// Creates a new Monitor
	pub fn new(stack: Stack) -> Monitor {
		Monitor {
			stack: stack,
			registers: Registers::new()
		}
	}

	/// Enters V86 mode at the specified address
	pub unsafe fn start(&self, address: u32) {
		let mut registers = V86Registers {
			gs: 8 * 4,
			fs: 8 * 4,
			ds: 8 * 4,
			es: 8 * 4,
			ss: 8 * 4,
			esp: self.stack.offset,
			eflags: 0b01000000000011110100000000000000,
			cs: 8 * 3,
			eip: address
		};

		crate::println!("Address {}", address);

		_enter_v8086(address);
	}

	/// Reads the memory offset at the provided seg:addr
	pub unsafe fn peek<T: IntegerValue>(&self, segment: u32, offset: u32) -> T {
		let logical = seg_off_to_log!(segment, offset);

		let ptr = logical as *const T;
		let value = *ptr;
		value
	}

	/// Writes to the memory offset at the provided seg:addr
	pub unsafe fn poke<T: IntegerValue>(&self, segment: u32, offset: u32, value: T) {
		let logical = seg_off_to_log!(segment, offset);

		let ptr = logical as *mut T;
		*ptr = value;
	}

	/// Reads one byte from the current EIP and increments it
	pub unsafe fn fetch(&mut self) -> u8 {
		let value = self.peek(self.registers.cs, self.registers.eip);
		self.registers.eip = (self.registers.eip + 1) & 0xFFFF;

		value
	}

	/// Pops a value from the v86 stack
	pub unsafe fn pop<T: IntegerValue>(&mut self) -> T {
		let value = self.peek(self.stack.segment, self.stack.offset);
		self.stack.offset = (self.stack.offset + (mem::size_of_val(&value) as u32)) & 0xFFFF;

		value
	}

	/// Pushes a value to the v86 stack
	pub unsafe fn push<T: IntegerValue>(&mut self, value: T) {
		self.stack.offset = (self.stack.offset - (mem::size_of_val(&value) as u32)) & 0xFFFF;
		self.poke(self.stack.segment, self.stack.offset, value);
	}

	/// Handles an interrupt using the BIOS IVT
	pub unsafe fn handle_interrupt(&mut self, int_number: u32) {
		// Push return IP, CS and EFLAGS onto the V86 stack
		self.push(self.registers.eflags);
		self.push(self.registers.cs);
		self.push(self.registers.eip);

		// Disable Interrupts
		self.registers.eflags &= !0x200;

		// Load new CS and IP from the IVT
		let ivt_offset = int_number * 4;
		self.registers.eip = (self.registers.eip & !0xFFFF) | self.peek::<u32>(0, ivt_offset);
		self.registers.cs = self.peek(0, ivt_offset + 2);
	}

	/// Executes an instruction
	pub unsafe fn emulate(&mut self) {
		let inital_eip = self.registers.eip;
		let mut prefix = 0;
		let mut instruction = 0;

		loop {
			instruction = self.fetch();

			match instruction {
				// Segment prefixes
				0x26 => prefix |= PFX_ES,
				0x2e => prefix |= PFX_CS,
				0x36 => prefix |= PFX_SS,
				0x3e => prefix |= PFX_DS,
				0x64 => prefix |= PFX_FS,
				0x65 => prefix |= PFX_GS,

				0x66 => prefix |= PFX_OP32,
				0x67 => prefix |= PFX_ADDR32,
				0xF0 => prefix |= PFX_LOCK,
				0xF2 => prefix |= PFX_REPNE,
				0xF3 => prefix |= PFX_REP,
				_ => break,
			}
		};

		match instruction {
			// PUSHF
			0x9C => {
				if (prefix & PFX_OP32) == PFX_OP32 {
					self.push(self.registers.eflags);
				} else {
					self.push(self.registers.eflags as u16);
				}
			},

			// POPF
			0x9D => {
				if (prefix & PFX_OP32) == PFX_OP32 {
					if self.registers.esp > 0xFFFC {
						return;
					}

					self.registers.eflags = self.pop();
				} else {
					if self.registers.esp > 0xFFFE {
						return;
					}

					self.registers.eflags = (self.registers.eflags & 0xFFFF0000) | self.pop::<u32>();
				}
			},

			// INT nn
			0xCD => {
				let interrupt_number = self.fetch() as u32;
				self.handle_interrupt(interrupt_number);
			},

			// IRET
			0xCF => {
				if (prefix & PFX_OP32) == PFX_OP32 {
					if self.registers.esp > 0xFFF4 {
						return;
					}

					self.registers.eip = self.pop();
					self.registers.cs = self.pop();
					self.registers.eflags = self.pop();
				} else {
					if self.registers.esp > 0xFFFA {
						return;
					}

					self.registers.eip = self.pop::<u16>() as u32;
					self.registers.cs = self.pop::<u16>() as u32;
					self.registers.eflags = self.pop::<u16>() as u32;
				}
			},

			// CLI & STI
			0xFA => self.registers.eflags &= !0x200,
			0xFB => self.registers.eflags |= 0x200,

			// Other
			_ => panic!("Unimplemented V8086 Instruction")
		}
	}
}