use crate::vga::{vga_colors::Color16Bit, Plane, Vga, VideoMode, VGA};
use spinning_top::SpinlockGuard;

const WIDTH: usize = 640;
const HEIGHT: usize = 480;

static PLANES: &'static [Plane] = &[Plane::Plane0, Plane::Plane1, Plane::Plane2, Plane::Plane3];

/// A basic interface for interacting with vga graphics mode 640x480x16
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// let graphics_mode = Graphics640x480x16::new();
/// graphics_mode.set_mode();
/// graphics_mode.clear_screen();
/// ```
pub struct Graphics640x480x16;

impl Graphics640x480x16 {
    /// Creates a new `Graphics640x480x16`.
    pub fn new() -> Graphics640x480x16 {
        Graphics640x480x16 {}
    }

    /// Clears the screen by setting all pixels to `Color16Bit::Black`.
    pub fn clear_screen(&self) {
        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                self.set_pixel(x, y, Color16Bit::Yellow);
            }
        }
    }

    /// Sets the given pixel at `(x, y)` to the given `color`.
    ///
    /// Panics if `x >= 640` or `y >= 480`.
    pub fn set_pixel(&self, x: usize, y: usize, color: Color16Bit) {
        assert!(x < WIDTH, "x >= {}", WIDTH);
        assert!(y < HEIGHT, "y >= {}", HEIGHT);
        let (mut vga, frame_buffer) = self.get_frame_buffer();
        let offset = (x / 8 + (WIDTH / 8) * y) as isize;

        // Store the current value for masking.
        let x = x & 7;
        let mask = 0x80 >> (x & 7);
        let mut plane_mask = 0x01;

        for plane in PLANES {
            vga.set_plane(*plane);
            let current_value = unsafe { frame_buffer.offset(offset).read_volatile() };
            let new_value = if plane_mask & color as u8 != 0 {
                current_value | mask
            } else {
                current_value & !mask
            };
            unsafe {
                frame_buffer.offset(offset).write_volatile(new_value);
            }
            plane_mask <<= 1;
        }
    }

    /// Sets the graphics device to `VideoMode::Mode640x480x16`.
    pub fn set_mode(&self) {
        VGA.lock().set_video_mode(VideoMode::Mode640x480x16);
    }

    /// Returns the start of the `FrameBuffer` as `*mut u8` as
    /// well as a lock to the vga driver. This ensures the vga
    /// driver stays locked while the frame buffer is in use.
    fn get_frame_buffer(&self) -> (SpinlockGuard<Vga>, *mut u8) {
        let mut vga = VGA.lock();
        let frame_buffer = vga.get_frame_buffer();
        (vga, u32::from(frame_buffer) as *mut u8)
    }
}
