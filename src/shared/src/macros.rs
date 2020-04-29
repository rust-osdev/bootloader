#[macro_export]
macro_rules! linker_symbol {
	($symbol_name:ident) => {unsafe {
	    let symbol_value: u32;

		llvm_asm!(concat!("lea eax, ", stringify!($symbol_name))
			: "={eax}"(symbol_value)
			::: "intel", "volatile");

		symbol_value
	}};
}
