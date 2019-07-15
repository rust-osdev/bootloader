use core::fmt::{Result, Write};
use core::sync::atomic::{AtomicUsize, Ordering};

const VGA_BUFFER: *mut u8 = 0xa0000 as *mut _;
const SCREEN_WIDTH: usize = 320;
const SCREEN_HEIGHT: usize = 200;

// must not be 0 so that we don't have a .bss section
pub static X_POS: AtomicUsize = AtomicUsize::new(1);
pub static Y_POS: AtomicUsize = AtomicUsize::new(1);

pub struct Printer;

impl Printer {
    pub fn clear_screen(&mut self) {
        for i in 0..(SCREEN_WIDTH * SCREEN_HEIGHT) {
            unsafe {
                VGA_BUFFER.offset(i as isize).write_volatile(0);
            }
        }

        X_POS.store(0, Ordering::SeqCst);
        Y_POS.store(0, Ordering::SeqCst);
    }

    fn newline(&mut self) {
        let y_pos = Y_POS.fetch_add(8, Ordering::SeqCst);
        X_POS.store(0, Ordering::SeqCst);
        if y_pos >= SCREEN_HEIGHT {
            self.clear_screen();
        }
    }

    fn write_char(&mut self, c: char) {
        use font8x8::UnicodeFonts;

        if c == '\n' {
            self.newline();
            return;
        }

        let x_pos = X_POS.fetch_add(8, Ordering::SeqCst);
        let y_pos = Y_POS.load(Ordering::SeqCst);

        match c {
            ' '..='~' => {
                let rendered = font8x8::BASIC_FONTS
                    .get(c)
                    .expect("character not found in basic font");
                for (y, byte) in rendered.iter().enumerate() {
                    for (x, bit) in (0..8).enumerate() {
                        if *byte & (1 << bit) == 0 {
                            continue;
                        }
                        let color = 0xf;
                        let idx = (y_pos + y) * SCREEN_WIDTH + x_pos + x;
                        unsafe {
                            VGA_BUFFER.offset(idx as isize).write_volatile(color);
                        }
                    }
                }
            }
            _ => panic!("unprintable character"),
        }

        if x_pos + 8 >= SCREEN_WIDTH {
            self.newline();
        }
    }
}

impl Write for Printer {
    fn write_str(&mut self, s: &str) -> Result {
        for c in s.chars() {
            self.write_char(c);
        }

        Ok(())
    }
}
