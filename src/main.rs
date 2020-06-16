#![no_std]
#![no_main]
#![feature(abi_efiapi)]

extern crate alloc;
extern crate rlibc;

use alloc::vec;
use core::mem;
use uefi::{
    prelude::{entry, Boot, Handle, ResultExt, Status, SystemTable},
    proto::console::gop::GraphicsOutput,
    table::boot::MemoryDescriptor,
};

#[entry]
fn efi_main(image: Handle, st: SystemTable<Boot>) -> Status {
    // Initialize utilities (logging, memory allocation...)
    uefi_services::init(&st).expect_success("Failed to initialize utilities");

    log::set_max_level(log::LevelFilter::Info);

    let stdout = st.stdout();
    stdout.reset(true).expect_success("failed to reset stdout");
    log::info!("Hello World from UEFI bootloader!");

    let boot_services = st.boot_services();
    let gop = boot_services
        .locate_protocol::<GraphicsOutput>()
        .expect_success("failed to locate gop");
    let gop = unsafe { &mut *gop.get() };

    // print available video modes
    for mode in gop.modes().map(|c| c.unwrap()) {
        log::trace!("Mode: {:x?}", mode.info());
    }

    let mode_info = gop.current_mode_info();
    let (width, height) = mode_info.resolution();
    log::info!("Active video Mode: {:x?}", mode_info);

    let mut framebuffer = gop.frame_buffer();

    let max_mmap_size =
        st.boot_services().memory_map_size() + 8 * mem::size_of::<MemoryDescriptor>();
    let mut mmap_storage = vec![0; max_mmap_size];

    log::trace!("exiting boot services");

    st.exit_boot_services(image, &mut mmap_storage)
        .expect_success("Failed to exit boot services");

    let fill_color = [0x00u8, 0x00, 0xff, 0x00];
    for col in 0..width {
        for row in 0..height {
            let index = row * mode_info.stride() + col;
            let byte_index = index * 4;
            unsafe {
                framebuffer.write_value(byte_index, fill_color);
            }
        }
    }

    loop {}
}
