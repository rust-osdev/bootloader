#![feature(slice_fill)]
#![no_std]

pub use logger::{FrameBufferInfo, PixelFormat};
use core::panic::PanicInfo;

mod load_kernel;
mod logger;

pub fn init_logger(framebuffer: &'static mut [u8], info: FrameBufferInfo) {
    let logger = logger::LOGGER.get_or_init(move || logger::LockedLogger::new(framebuffer, info));
    log::set_logger(logger).expect("logger already set");
    log::set_max_level(log::LevelFilter::Trace);
}

pub fn load_kernel(kernel: &'static [u8]) -> ! {
    load_kernel::load_kernel(kernel).expect("Failed to parse kernel");
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe { logger::LOGGER.get().map(|l| l.force_unlock()) };
    log::error!("{}", info);
    loop {}
}
