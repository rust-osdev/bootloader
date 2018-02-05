use core::fmt::{Write, Result};
use core::slice;
use spin::Mutex;

pub static PRINTER: Mutex<Printer> = Mutex::new(Printer::new());

pub struct Printer {
    index: usize,
}

impl Printer {
    pub const fn new() -> Printer {
        Printer {
            index: 0,
        }
    }
}

impl Write for Printer {
    fn write_str(&mut self, s: &str) -> Result {
        const VGA_BUFFER: *mut u8 = 0xb8000 as *mut _;
        let vga_buffer = unsafe { slice::from_raw_parts_mut(VGA_BUFFER, 80 * 25 * 2) };
        for byte in s.bytes() {
            vga_buffer[self.index] = byte;
            vga_buffer[self.index + 1] = 0x4f;
            self.index += 2;
        }

        Ok(())
    }
}