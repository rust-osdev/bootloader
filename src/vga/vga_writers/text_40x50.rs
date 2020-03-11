use super::ScreenCharacter;
use crate::vga::{
    vga_colors::{Color16Bit, TextModeColor},
    VideoMode, VGA,
};

const WIDTH: usize = 40;
const HEIGHT: usize = 50;
const SCREEN_SIZE: usize = WIDTH * HEIGHT;

static BLANK_CHARACTER: ScreenCharacter = ScreenCharacter {
    character: b' ',
    color: TextModeColor::new(Color16Bit::Yellow, Color16Bit::Black),
};

/// A basic interface for interacting with vga text mode 80x50
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// let text_mode = Text40x50::new();
/// text_mode.set_mode();
/// text_mode.clear_screen();
/// ```
pub struct Text40x50;

impl Text40x50 {
    /// Creates a new `Text40x50`.
    pub fn new() -> Text40x50 {
        Text40x50 {}
    }

    /// Clears the screen by setting all cells to `b' '` with
    /// a background color of `Color16Bit::Black` and a foreground
    /// color of `Color16Bit::Yellow`.
    pub fn clear_screen(&self) {
        let frame_buffer = self.get_frame_buffer();
        for i in 0..SCREEN_SIZE {
            unsafe {
                frame_buffer
                    .offset(i as isize)
                    .write_volatile(BLANK_CHARACTER);
            }
        }
    }

    /// Prints the given `character` and `color` at `(x, y)`.
    ///
    /// Panics if `x >= 40` or `y >= 50`.
    pub fn write_character(&self, x: usize, y: usize, character: u8, color: TextModeColor) {
        assert!(x < WIDTH, "x >= {}", WIDTH);
        assert!(y < HEIGHT, "y >= {}", HEIGHT);
        let frame_buffer = self.get_frame_buffer();
        let offset = (WIDTH * y + x) as isize;
        unsafe {
            frame_buffer
                .offset(offset)
                .write_volatile(ScreenCharacter { character, color });
        }
    }

    /// Sets the graphics device to `VideoMode::Mode40x50`.
    pub fn set_mode(&self) {
        VGA.lock().set_video_mode(VideoMode::Mode40x50);
    }

    /// Returns the start of the `FrameBuffer` as `*mut ScreenCharacter`.
    fn get_frame_buffer(&self) -> *mut ScreenCharacter {
        u32::from(VGA.lock().get_frame_buffer()) as *mut ScreenCharacter
    }
}
