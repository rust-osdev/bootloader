#![no_std]
#![no_main]
#![feature(abi_efiapi)]

static KERNEL: PageAligned<[u8; 137224]> = PageAligned(*include_bytes!(
    "../../blog_os/post-01/target/x86_64-blog_os/debug/blog_os"
));

#[repr(align(4096))]
struct PageAligned<T>(T);

extern crate rlibc;

use core::{mem, slice};
use uefi::{
    prelude::{entry, Boot, Handle, ResultExt, Status, SystemTable},
    proto::console::gop::{GraphicsOutput, PixelFormat},
    table::boot::{MemoryDescriptor, MemoryType},
};

#[entry]
fn efi_main(image: Handle, st: SystemTable<Boot>) -> Status {
    init_logger(&st);
    log::info!("Hello World from UEFI bootloader!");

    let mmap_storage = {
        let max_mmap_size =
            st.boot_services().memory_map_size() + 8 * mem::size_of::<MemoryDescriptor>();
        let ptr = st
            .boot_services()
            .allocate_pool(MemoryType::LOADER_DATA, max_mmap_size)?
            .log();
        unsafe { slice::from_raw_parts_mut(ptr, max_mmap_size) }
    };

    log::trace!("exiting boot services");
    let (_st, memory_map) = st
        .exit_boot_services(image, mmap_storage)
        .expect_success("Failed to exit boot services");

    bootloader_lib::load_kernel(&KERNEL.0)
}

fn init_logger(st: &SystemTable<Boot>) {
    let gop = st
        .boot_services()
        .locate_protocol::<GraphicsOutput>()
        .expect_success("failed to locate gop");
    let gop = unsafe { &mut *gop.get() };

    let mode_info = gop.current_mode_info();
    let mut framebuffer = gop.frame_buffer();
    let slice = unsafe { slice::from_raw_parts_mut(framebuffer.as_mut_ptr(), framebuffer.size()) };
    let info = bootloader_lib::FrameBufferInfo {
        horizontal_resolution: mode_info.resolution().0,
        vertical_resolution: mode_info.resolution().1,
        pixel_format: match mode_info.pixel_format() {
            PixelFormat::RGB => bootloader_lib::PixelFormat::BGR,
            PixelFormat::BGR => bootloader_lib::PixelFormat::BGR,
            PixelFormat::Bitmask | PixelFormat::BltOnly => {
                panic!("Bitmask and BltOnly framebuffers are not supported")
            }
        },
        stride: mode_info.stride(),
    };

    bootloader_lib::init_logger(slice, info);
}
