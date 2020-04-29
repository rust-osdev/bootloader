use core::{fmt, fmt::Write};

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut writer = $crate::console::Writer {};
        let _ = writer.write_fmt(format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

pub struct Writer {}

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &byte in s.as_bytes() {
            let _ = self.write_byte(byte);
        }

        Ok(())
    }
}

impl Writer {
    #[inline(always)]
    fn write_byte(&mut self, c: u8) {
        let ax = u16::from(c) | 0x0e00;

        unsafe {
            llvm_asm!("int 0x10" :: "{ax}"(ax), "{bx}"(0) :: "intel", "volatile");
        }
    }
}
