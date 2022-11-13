use bootloader_x86_64_bios_common::{racy_cell::RacyCell, BiosFramebufferInfo, PixelFormat};
use core::{fmt, ptr};
use noto_sans_mono_bitmap::{get_bitmap, BitmapChar, BitmapHeight, FontWeight};

static WRITER: RacyCell<Option<ScreenWriter>> = RacyCell::new(None);
pub struct Writer;

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let writer = unsafe { WRITER.get_mut() }.as_mut().unwrap();
        writer.write_str(s)
    }
}

pub fn init(info: BiosFramebufferInfo) {
    let framebuffer = unsafe {
        core::slice::from_raw_parts_mut(
            info.region.start as *mut u8,
            info.region.len.try_into().unwrap(),
        )
    };
    let writer = ScreenWriter::new(framebuffer, info);
    *unsafe { WRITER.get_mut() } = Some(writer);
}

/// Additional vertical space between lines
const LINE_SPACING: usize = 0;

struct ScreenWriter {
    framebuffer: &'static mut [u8],
    info: BiosFramebufferInfo,
    x_pos: usize,
    y_pos: usize,
}

impl ScreenWriter {
    pub fn new(framebuffer: &'static mut [u8], info: BiosFramebufferInfo) -> Self {
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
        self.y_pos += 14 + LINE_SPACING;
        self.carriage_return()
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
        self.info.width.into()
    }

    fn height(&self) -> usize {
        self.info.height.into()
    }

    fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            '\r' => self.carriage_return(),
            c => {
                let bitmap_char = get_bitmap(c, FontWeight::Regular, BitmapHeight::Size14).unwrap();
                if self.x_pos + bitmap_char.width() > self.width() {
                    self.newline();
                }
                if self.y_pos + bitmap_char.height() > self.height() {
                    self.clear();
                }
                self.write_rendered_char(bitmap_char);
            }
        }
    }

    fn write_rendered_char(&mut self, rendered_char: BitmapChar) {
        for (y, row) in rendered_char.bitmap().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                self.write_pixel(self.x_pos + x, self.y_pos + y, *byte);
            }
        }
        self.x_pos += rendered_char.width();
    }

    fn write_pixel(&mut self, x: usize, y: usize, intensity: u8) {
        let pixel_offset = y * usize::from(self.info.stride) + x;
        let color = match self.info.pixel_format {
            PixelFormat::Rgb => [intensity, intensity, intensity / 2, 0],
            PixelFormat::Bgr => [intensity / 2, intensity, intensity, 0],
            other => {
                // set a supported (but invalid) pixel format before panicking to avoid a double
                // panic; it might not be readable though
                self.info.pixel_format = PixelFormat::Rgb;
                panic!("pixel format {:?} not supported in logger", other)
            }
        };
        let bytes_per_pixel = self.info.bytes_per_pixel;
        let byte_offset = pixel_offset * usize::from(bytes_per_pixel);
        self.framebuffer[byte_offset..(byte_offset + usize::from(bytes_per_pixel))]
            .copy_from_slice(&color[..usize::from(bytes_per_pixel)]);
        let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
    }
}

unsafe impl Send for ScreenWriter {}
unsafe impl Sync for ScreenWriter {}

impl fmt::Write for ScreenWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}
