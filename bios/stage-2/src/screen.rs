use core::{arch::asm, fmt::Write};

pub fn print_char(c: u8) {
    let ax = u16::from(c) | 0x0e00;
    unsafe {
        asm!("push bx", "mov bx, 0", "int 0x10", "pop bx", in("ax") ax);
    }
}

pub fn print_str(s: &str) {
    for c in s.chars() {
        if c.is_ascii() {
            print_char(c as u8);
            if c == '\n' {
                print_char(b'\r');
            }
        } else {
            print_char(b'X');
        }
    }
}

pub struct Writer;

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        print_str(s);
        Ok(())
    }
}

#[cfg(all(not(test), target_os = "none"))]
#[panic_handler]
pub fn panic(info: &core::panic::PanicInfo) -> ! {
    let _ = write!(Writer, "\nPANIC: ");
    if let Some(location) = info.location() {
        let _ = writeln!(Writer, "{location} ");
    }
    let _ = writeln!(Writer, " {info}");

    loop {
        unsafe {
            asm!("hlt");
        };
    }
}
