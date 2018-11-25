use core::fmt::{Result, Write};
use core::slice;
use core::sync::atomic::{AtomicUsize, Ordering};

const VGA_BUFFER: *mut u8 = 0xb8000 as *mut _;
const SCREEN_SIZE: usize = 80 * 25;

pub static CURRENT_OFFSET: AtomicUsize = AtomicUsize::new(160);

pub struct Printer;

impl Printer {
    pub fn clear_screen(&mut self) {
        let vga_buffer = Self::vga_buffer();
        for byte in vga_buffer {
            *byte = 0;
        }
        CURRENT_OFFSET.store(0, Ordering::Relaxed);
    }

    fn vga_buffer() -> &'static mut [u8] {
        unsafe { slice::from_raw_parts_mut(VGA_BUFFER, SCREEN_SIZE * 2) }
    }
}

impl Write for Printer {
    fn write_str(&mut self, s: &str) -> Result {
        let vga_buffer = Self::vga_buffer();
        for byte in s.bytes() {
            let index = CURRENT_OFFSET.fetch_add(2, Ordering::Relaxed);
            vga_buffer[index] = byte;
            vga_buffer[index + 1] = 0x4f;
        }

        Ok(())
    }
}
