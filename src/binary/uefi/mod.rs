pub use logger::{FrameBufferInfo, PixelFormat};
use x86_64::{VirtAddr, structures::paging::{FrameAllocator, MapperAllSizes, Size4KiB}};

mod load_kernel;
pub mod logger;
mod memory_descriptor;

pub fn init_logger(framebuffer: &'static mut [u8], info: FrameBufferInfo) {
    let logger = logger::LOGGER.get_or_init(move || logger::LockedLogger::new(framebuffer, info));
    log::set_logger(logger).expect("logger already set");
    log::set_max_level(log::LevelFilter::Trace);
}

pub fn load_kernel(
    kernel: &'static [u8],
    page_table: &mut impl MapperAllSizes,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> VirtAddr {
    load_kernel::load_kernel(kernel, page_table, frame_allocator).expect("Failed to parse kernel")
}
