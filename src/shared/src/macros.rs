#[macro_export]
macro_rules! linker_symbol {
	($symbol_name:ident) => {unsafe {
	    let symbol_value: u32;

		asm!(
			concat!("lea {}, ", stringify!($symbol_name)),
			out(reg) symbol_value
		);

		symbol_value
	}};
}
