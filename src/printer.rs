use core::fmt::{Write, Result};
use core::slice;
use spin::Mutex;

const VGA_BUFFER: *mut u8 = 0xb8000 as *mut _;
const SCREEN_SIZE: usize = 80 * 25;

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

    pub fn clear_screen(&mut self) {
        let vga_buffer = Self::vga_buffer();
        for byte in vga_buffer {
            *byte = 0;
        }
        self.index = 0;
    }

    fn vga_buffer() -> &'static mut [u8] {
         unsafe { slice::from_raw_parts_mut(VGA_BUFFER, SCREEN_SIZE * 2) }
    }
}

impl Write for Printer {
    fn write_str(&mut self, s: &str) -> Result {
        let vga_buffer = Self::vga_buffer();
        for byte in s.bytes() {
            vga_buffer[self.index] = byte;
            vga_buffer[self.index + 1] = 0x4f;
            self.index += 2;
        }

        Ok(())
    }
}
