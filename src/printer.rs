use core::fmt;
use spin::Mutex;
#[cfg(feature = "serial")]
use uart_16550::SerialPort;

macro_rules! println {
    () => (print!("\n"));
    ($fmt: expr) => (use ::core::fmt::Write; $crate::printer::PRINTER.lock().write_fmt(format_args!(concat!($fmt, "\n"))).unwrap());
    ($fmt: expr, $($arg: tt)*) => (use ::core::fmt::Write; $crate::printer::PRINTER.lock().write_fmt(format_args!(concat!($fmt, "\n"), $($arg)*)).unwrap());
}

lazy_static! {
    pub static ref PRINTER: Mutex<Printer> = Mutex::new(Printer::new());
}

pub struct Printer {
    #[cfg(feature = "vga")]
    vga_buffer: VgaBuffer,

    #[cfg(feature = "serial")]
    serial_port: SerialPort,
}

impl Printer {
    fn new() -> Printer {
        #[cfg(feature = "serial")]
        let serial_port = {
            let mut port = SerialPort::new(0x3f8);
            port.init();
            port
        };

        Printer {
            #[cfg(feature = "vga")]
            vga_buffer: VgaBuffer::new(),

            #[cfg(feature = "serial")]
            serial_port,
        }
    }
}

impl fmt::Write for Printer {
    fn write_str(&mut self, string: &str) -> fmt::Result {
        for byte in string.bytes() {
            #[cfg(feature = "vga")]
            self.vga_buffer.print_byte(byte);

            #[cfg(feature = "serial")]
            match byte {
                b'\n' => {
                    self.serial_port.send(b'\n');
                    self.serial_port.send(b'\r');
                }

                _ => {
                    self.serial_port.send(byte);
                }
            }
        }

        Ok(())
    }
}

#[cfg(feature = "vga")]
struct VgaBuffer {
    row: usize,
    column: usize,
}

#[cfg(feature = "vga")]
impl VgaBuffer {
    pub fn new() -> VgaBuffer {
        let mut vga = VgaBuffer {
            row: 0,
            column: 0,
        };

        for byte in vga.buffer() {
            *byte = 0;
        }

        vga
    }

    fn print_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.row += 1;
                self.column = 0;

                // TODO: if we've run out of space, scroll the terminal up
            }

            _ => {
                let vga_buffer = self.buffer();
                vga_buffer[(self.row * 80 + self.column) * 2] = byte;
                vga_buffer[(self.row * 80 + self.column) * 2 + 1] = 0xb;
                self.column += 1;
            }
        }
    }

    fn buffer(&mut self) -> &'static mut [u8] {
        const VGA_BUFFER: *mut u8 = 0xb8000 as *mut _;
        const BUFFER_WIDTH: usize = 80;
        const BUFFER_HEIGHT: usize = 25;

        unsafe {
            ::core::slice::from_raw_parts_mut(VGA_BUFFER, BUFFER_WIDTH * BUFFER_HEIGHT)
        }
    }
}
