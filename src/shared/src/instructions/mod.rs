global_asm!(include_str!("assembly.s"));

extern "C" {
	pub fn retf(cs: u32, eip: u32);
}