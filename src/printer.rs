use core::fmt;
use core::slice;
use spin::Mutex;

macro_rules! println {
    () => (print!("\n"));
    ($fmt: expr) => ($crate::printer::PRINTER.lock().print(format_args!(concat!($fmt, "\n"))));
    ($fmt: expr, $($arg: tt)*) => ($crate::printer::PRINTER.lock().print(format_args!(concat!($fmt, "\n"), $($arg)*)));
}

const VGA_BUFFER: *mut u8 = 0xb8000 as *mut _;
const BUFFER_WIDTH: usize = 80;
const BUFFER_HEIGHT: usize = 25;

pub static PRINTER: Mutex<Printer> = Mutex::new(Printer::new());

pub struct Printer {
    row: usize,
    column: usize,
}

impl Printer {
    const fn new() -> Printer {
        Printer {
            row: 0,
            column: 0,
        }
    }

    pub fn clear_screen(&mut self) {
        for byte in self.vga_buffer() {
            *byte = 0;
        }

        self.row = 0;
        self.column = 0;
    }

    pub fn print(&mut self, args: fmt::Arguments) {
        use core::fmt::Write;
        self.write_fmt(args).unwrap();
    }

    fn vga_buffer(&mut self) -> &'static mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(VGA_BUFFER, BUFFER_WIDTH * BUFFER_HEIGHT)
        }
    }
}

impl fmt::Write for Printer {
    fn write_str(&mut self, string: &str) -> fmt::Result {
        for byte in string.bytes() {
            match byte {
                b'\n' => {
                    self.row += 1;
                    self.column = 0;

                    // TODO: VGA: if we've run out of space, scroll the terminal up
                }

                _ => {
                    let vga_buffer = self.vga_buffer();
                    vga_buffer[(self.row * 80 + self.column) * 2] = byte;
                    vga_buffer[(self.row * 80 + self.column) * 2 + 1] = 0xb;
                    self.column += 1;
                }
            }
        }

        Ok(())
    }
}
