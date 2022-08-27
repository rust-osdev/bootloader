use core::fmt::{Result, Write};
use core::sync::atomic::{AtomicUsize, Ordering};

const VGA_BUFFER: *mut u8 = 0xb8000 as *mut _;
const SCREEN_WIDTH: usize = 80;
const SCREEN_SIZE: usize = SCREEN_WIDTH * 25;

pub static CURRENT_OFFSET: AtomicUsize = AtomicUsize::new(0);

pub struct Writer;

impl Writer {
    pub fn clear_screen(&mut self) {
        for i in 0..(SCREEN_SIZE * 2) {
            unsafe {
                VGA_BUFFER.offset(i as isize).write_volatile(0);
            }
        }

        CURRENT_OFFSET.store(0, Ordering::Relaxed);
    }
}

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> Result {
        for byte in s.bytes() {
            if byte == b'\n' {
                let current = CURRENT_OFFSET.load(Ordering::Relaxed);
                let bytes_per_line = SCREEN_WIDTH * 2;
                let offset = current % bytes_per_line;
                CURRENT_OFFSET.fetch_add(bytes_per_line - offset, Ordering::Relaxed) as isize;
                continue;
            }
            let index = CURRENT_OFFSET.fetch_add(2, Ordering::Relaxed) as isize;

            unsafe {
                VGA_BUFFER.offset(index).write_volatile(byte);
                VGA_BUFFER.offset(index + 1).write_volatile(0x1f);
            }
        }

        Ok(())
    }
}
