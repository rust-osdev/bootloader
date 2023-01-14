use serde::Deserialize;

#[derive(Deserialize)]
pub struct BootloaderConfigFile {
    /// Configuration for the frame buffer that can be used by the kernel to display pixels
    /// on the screen.
    #[serde(default)]
    pub frame_buffer: FrameBuffer,

    /// Configuration for changing the level of the filter of the messages that are shown in the
    /// screen when booting. The default is 'Trace'.
    #[serde(default)]
    pub log_level: LevelFilter,

    /// Whether the bootloader should print log messages to the framebuffer when booting.
    ///
    /// Enabled by default.
    #[serde(default)]
    pub frame_buffer_logger_status: LoggerStatus,

    /// Whether the bootloader should print log messages to the serial port when booting.
    ///
    /// Enabled by default.
    #[serde(default)]
    pub serial_logger_status: LoggerStatus,
}

impl Default for BootloaderConfigFile {
    fn default() -> Self {
        Self {
            frame_buffer: Default::default(),
            log_level: Default::default(),
            frame_buffer_logger_status: Default::default(),
            serial_logger_status: Default::default(),
        }
    }
}

impl BootloaderConfigFile {
    pub fn deserialize<'a>(serialized: Option<&'a mut [u8]>) -> Self {
        match serialized {
            Some(json) => return serde_json_core::from_slice(&json).unwrap().0,
            None => return Default::default(),
        }
    }
}

/// Configuration for the frame buffer used for graphical output.
#[derive(Deserialize, Debug, Default, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub struct FrameBuffer {
    /// Instructs the bootloader to set up a framebuffer format that has at least the given height.
    ///
    /// If this is not possible, the bootloader will fall back to a smaller format.
    pub minimum_framebuffer_height: Option<u64>,
    /// Instructs the bootloader to set up a framebuffer format that has at least the given width.
    ///
    /// If this is not possible, the bootloader will fall back to a smaller format.
    pub minimum_framebuffer_width: Option<u64>,
}

impl FrameBuffer {
    #[cfg(test)]
    fn random() -> FrameBuffer {
        Self {
            minimum_framebuffer_height: if rand::random() {
                Option::Some(rand::random())
            } else {
                Option::None
            },
            minimum_framebuffer_width: if rand::random() {
                Option::Some(rand::random())
            } else {
                Option::None
            },
        }
    }
}

/// An enum representing the available verbosity level filters of the logger.
///
/// Based on
/// https://github.com/rust-lang/log/blob/dc32ab999f52805d5ce579b526bd9d9684c38d1a/src/lib.rs#L552-565
#[derive(Deserialize, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LevelFilter {
    /// A level lower than all log levels.
    Off,
    /// Corresponds to the `Error` log level.
    Error,
    /// Corresponds to the `Warn` log level.
    Warn,
    /// Corresponds to the `Info` log level.
    Info,
    /// Corresponds to the `Debug` log level.
    Debug,
    /// Corresponds to the `Trace` log level.
    Trace,
}

impl Default for LevelFilter {
    fn default() -> Self {
        Self::Trace
    }
}

/// An enum for enabling or disabling the different methods for logging.
#[derive(Deserialize, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LoggerStatus {
    /// This method of logging is disabled
    Disable,
    /// This method of logging is enabled
    Enable,
}

impl Default for LoggerStatus {
    fn default() -> Self {
        Self::Enable
    }
}
