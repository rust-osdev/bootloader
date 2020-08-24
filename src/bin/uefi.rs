#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![feature(asm)]
#![feature(unsafe_block_in_unsafe_fn)]
#![feature(min_const_generics)]
#![feature(maybe_uninit_extra)]
#![feature(maybe_uninit_slice_assume_init)]
#![deny(unsafe_op_in_unsafe_fn)]

// Defines the constants `KERNEL_BYTES` (array of `u8`) and `KERNEL_SIZE` (`usize`).
include!(concat!(env!("OUT_DIR"), "/kernel_info.rs"));

static KERNEL: PageAligned<[u8; KERNEL_SIZE]> = PageAligned(KERNEL_BYTES);

#[repr(align(4096))]
struct PageAligned<T>(T);

extern crate rlibc;

use bootloader::binary::legacy_memory_region::LegacyFrameAllocator;
use core::{mem, panic::PanicInfo, slice};
use uefi::{
    prelude::{entry, Boot, Handle, ResultExt, Status, SystemTable},
    proto::console::gop::{GraphicsOutput, PixelFormat},
    table::boot::{MemoryDescriptor, MemoryType},
};
use x86_64::{
    structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

#[entry]
fn efi_main(image: Handle, st: SystemTable<Boot>) -> Status {
    let (framebuffer_addr, framebuffer_size) = init_logger(&st);
    log::info!("Hello World from UEFI bootloader!");
    log::info!("Using framebuffer at {:#x}", framebuffer_addr);

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

    let mut frame_allocator = LegacyFrameAllocator::new(memory_map.copied());

    let page_tables = create_page_tables(&mut frame_allocator);

    bootloader::binary::load_and_switch_to_kernel(
        &KERNEL.0,
        frame_allocator,
        page_tables,
        framebuffer_addr,
        framebuffer_size,
    );
}

/// Creates page table abstraction types for both the bootloader and kernel page tables.
fn create_page_tables(
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> bootloader::binary::PageTables {
    // UEFI identity-maps all memory, so the offset between physical and virtual addresses is 0
    let phys_offset = VirtAddr::new(0);

    // copy the currently active level 4 page table, because it might be read-only
    log::trace!("switching to new level 4 table");
    let bootloader_page_table = {
        let old_frame = x86_64::registers::control::Cr3::read().0;
        let old_table: *const PageTable =
            (phys_offset + old_frame.start_address().as_u64()).as_ptr();
        let new_frame = frame_allocator
            .allocate_frame()
            .expect("Failed to allocate frame for new level 4 table");
        let new_table: *mut PageTable =
            (phys_offset + new_frame.start_address().as_u64()).as_mut_ptr();
        // copy the table to the new frame
        unsafe { core::ptr::copy_nonoverlapping(old_table, new_table, 1) };
        // the tables are now identical, so we can just load the new one
        unsafe {
            x86_64::registers::control::Cr3::write(
                new_frame,
                x86_64::registers::control::Cr3Flags::empty(),
            );
            OffsetPageTable::new(&mut *new_table, phys_offset)
        }
    };

    // create a new page table hierarchy for the kernel
    let (kernel_page_table, kernel_level_4_frame) = {
        // get an unused frame for new level 4 page table
        let frame: PhysFrame = frame_allocator.allocate_frame().expect("no unused frames");
        log::info!("New page table at: {:#?}", &frame);
        // get the corresponding virtual address
        let addr = phys_offset + frame.start_address().as_u64();
        // initialize a new page table
        let ptr = addr.as_mut_ptr();
        unsafe { *ptr = PageTable::new() };
        let level_4_table = unsafe { &mut *ptr };
        (
            unsafe { OffsetPageTable::new(level_4_table, phys_offset) },
            frame,
        )
    };

    bootloader::binary::PageTables {
        bootloader: bootloader_page_table,
        kernel: kernel_page_table,
        kernel_level_4_frame,
    }
}

fn init_logger(st: &SystemTable<Boot>) -> (PhysAddr, usize) {
    let gop = st
        .boot_services()
        .locate_protocol::<GraphicsOutput>()
        .expect_success("failed to locate gop");
    let gop = unsafe { &mut *gop.get() };

    let mode_info = gop.current_mode_info();
    let mut framebuffer = gop.frame_buffer();
    let slice = unsafe { slice::from_raw_parts_mut(framebuffer.as_mut_ptr(), framebuffer.size()) };
    let info = bootloader::binary::logger::FrameBufferInfo {
        horizontal_resolution: mode_info.resolution().0,
        vertical_resolution: mode_info.resolution().1,
        pixel_format: match mode_info.pixel_format() {
            PixelFormat::RGB => bootloader::binary::logger::PixelFormat::BGR,
            PixelFormat::BGR => bootloader::binary::logger::PixelFormat::BGR,
            PixelFormat::Bitmask | PixelFormat::BltOnly => {
                panic!("Bitmask and BltOnly framebuffers are not supported")
            }
        },
        stride: mode_info.stride(),
    };

    bootloader::binary::init_logger(slice, info);

    (
        PhysAddr::new(framebuffer.as_mut_ptr() as u64),
        framebuffer.size(),
    )
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        bootloader::binary::logger::LOGGER
            .get()
            .map(|l| l.force_unlock())
    };
    log::error!("{}", info);
    loop {
        unsafe { asm!("cli; hlt") };
    }
}
