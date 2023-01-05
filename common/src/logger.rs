use crate::{framebuffer::FrameBufferWriter, serial::SerialPort};
use bootloader_api::{config::LoggerStatus, info::FrameBufferInfo};
use conquer_once::spin::OnceCell;
use core::fmt::Write;
use spinning_top::Spinlock;

/// The global logger instance used for the `log` crate.
pub static LOGGER: OnceCell<LockedLogger> = OnceCell::uninit();

/// A [`FrameBufferWriter`] instance protected by a spinlock.
pub struct LockedLogger(
    Spinlock<FrameBufferWriter>,
    Spinlock<SerialPort>,
    LoggerStatus,
    LoggerStatus,
);

impl LockedLogger {
    /// Create a new instance that logs to the given framebuffer.
    pub fn new(
        framebuffer: &'static mut [u8],
        info: FrameBufferInfo,
        frame_buffer_logger_status: LoggerStatus,
        serial_logger_status: LoggerStatus,
    ) -> Self {
        LockedLogger(
            Spinlock::new(FrameBufferWriter::new(framebuffer, info)),
            Spinlock::new(SerialPort::new()),
            frame_buffer_logger_status,
            serial_logger_status,
        )
    }

    /// Force-unlocks the logger to prevent a deadlock.
    ///
    /// ## Safety
    /// This method is not memory safe and should be only used when absolutely necessary.
    pub unsafe fn force_unlock(&self) {
        unsafe {
            self.0.force_unlock();
            self.1.force_unlock();
        };
    }
}

impl log::Log for LockedLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.2 == LoggerStatus::Enable {
            let mut logger = self.0.lock();
            writeln!(logger, "{:5}: {}", record.level(), record.args()).unwrap();
        }
        if self.3 == LoggerStatus::Enable {
            let mut serial = self.1.lock();
            writeln!(serial, "{:5}: {}", record.level(), record.args()).unwrap();
        }
    }

    fn flush(&self) {}
}
