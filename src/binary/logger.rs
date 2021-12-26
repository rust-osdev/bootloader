use bootloader_api::info::{FrameBufferInfo, PixelFormat};
use conquer_once::spin::OnceCell;
use core::{
    fmt::{self, Write},
    ptr,
};
use font8x8::UnicodeFonts;
use spinning_top::Spinlock;

/// The global logger instance used for the `log` crate.
pub static LOGGER: OnceCell<LockedLogger> = OnceCell::uninit();

/// A [`Logger`] instance protected by a spinlock.
pub struct LockedLogger(Spinlock<Logger>);

/// Additional vertical space between lines
const LINE_SPACING: usize = 0;
/// Additional vertical space between separate log messages
const LOG_SPACING: usize = 2;

impl LockedLogger {
    /// Create a new instance that logs to the given framebuffer.
    pub fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        LockedLogger(Spinlock::new(Logger::new(framebuffer, info)))
    }

    /// Force-unlocks the logger to prevent a deadlock.
    ///
    /// This method is not memory safe and should be only used when absolutely necessary.
    pub unsafe fn force_unlock(&self) {
        unsafe { self.0.force_unlock() };
    }
}

impl log::Log for LockedLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let mut logger = self.0.lock();
        writeln!(logger, "{}:    {}", record.level(), record.args()).unwrap();
        logger.add_vspace(LOG_SPACING);
    }

    fn flush(&self) {}
}

/// Allows logging text to a pixel-based framebuffer.
pub struct Logger {
    framebuffer: &'static mut [u8],
    info: FrameBufferInfo,
    x_pos: usize,
    y_pos: usize,
}

impl Logger {
    /// Creates a new logger that uses the given framebuffer.
    pub fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        let mut logger = Self {
            framebuffer,
            info,
            x_pos: 0,
            y_pos: 0,
        };
        logger.clear();
        logger
    }

    fn newline(&mut self) {
        self.y_pos += 8 + LINE_SPACING;
        self.carriage_return()
    }

    fn add_vspace(&mut self, space: usize) {
        self.y_pos += space;
    }

    fn carriage_return(&mut self) {
        self.x_pos = 0;
    }

    /// Erases all text on the screen.
    pub fn clear(&mut self) {
        self.x_pos = 0;
        self.y_pos = 0;
        self.framebuffer.fill(0);
    }

    fn width(&self) -> usize {
        self.info.horizontal_resolution
    }

    fn height(&self) -> usize {
        self.info.vertical_resolution
    }

    fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            '\r' => self.carriage_return(),
            c => {
                if self.x_pos >= self.width() {
                    self.newline();
                }
                if self.y_pos >= (self.height() - 8) {
                    self.clear();
                }
                let rendered = font8x8::BASIC_FONTS
                    .get(c)
                    .expect("character not found in basic font");
                self.write_rendered_char(rendered);
            }
        }
    }

    fn write_rendered_char(&mut self, rendered_char: [u8; 8]) {
        for (y, byte) in rendered_char.iter().enumerate() {
            for (x, bit) in (0..8).enumerate() {
                let alpha = if *byte & (1 << bit) == 0 { 0 } else { 255 };
                self.write_pixel(self.x_pos + x, self.y_pos + y, alpha);
            }
        }
        self.x_pos += 8;
    }

    fn write_pixel(&mut self, x: usize, y: usize, intensity: u8) {
        let pixel_offset = y * self.info.stride + x;
        let color = match self.info.pixel_format {
            PixelFormat::RGB => [intensity, intensity, intensity / 2, 0],
            PixelFormat::BGR => [intensity / 2, intensity, intensity, 0],
            PixelFormat::U8 => [if intensity > 200 { 0xf } else { 0 }, 0, 0, 0],
        };
        let bytes_per_pixel = self.info.bytes_per_pixel;
        let byte_offset = pixel_offset * bytes_per_pixel;
        self.framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
            .copy_from_slice(&color[..bytes_per_pixel]);
        let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
    }
}

unsafe impl Send for Logger {}
unsafe impl Sync for Logger {}

impl fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}
