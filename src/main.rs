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
    table::boot::{MemoryDescriptor, MemoryMapIter, MemoryType},
};
use x86_64::{
    structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

const PAGE_SIZE: u64 = 4096;

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

    let mut frame_allocator = UefiFrameAllocator::new(memory_map);

    let mut page_table = {
        // UEFI identity maps all memory, so physical memory offset is 0
        let phys_offset = VirtAddr::new(0);
        // get an unused frame for new level 4 page table
        let frame: PhysFrame = frame_allocator.allocate_frame().expect("no unused frames");
        // get the corresponding virtual address
        let addr = phys_offset + frame.start_address().as_u64();
        // initialize a new page table
        let ptr = addr.as_mut_ptr();
        unsafe { *ptr = PageTable::new() };
        let level_4_table = unsafe { &mut *ptr };
        unsafe { OffsetPageTable::new(level_4_table, phys_offset) }
    };

    bootloader_lib::load_kernel(&KERNEL.0, &mut page_table, &mut frame_allocator);

    loop {
        unsafe { asm!("cli; hlt") };
    }
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

struct UefiFrameAllocator<'a> {
    memory_map: MemoryMapIter<'a>,
    current_descriptor: Option<&'a MemoryDescriptor>,
    next_frame: PhysFrame,
}

impl<'a> UefiFrameAllocator<'a> {
    fn new(memory_map: MemoryMapIter<'a>) -> Self {
        Self {
            memory_map,
            current_descriptor: None,
            next_frame: PhysFrame::containing_address(PhysAddr::new(0)),
        }
    }

    fn allocate_frame_from_descriptor(
        &mut self,
        descriptor: &MemoryDescriptor,
    ) -> Option<PhysFrame> {
        let start_addr = PhysAddr::new(descriptor.phys_start);
        let start_frame: PhysFrame = PhysFrame::containing_address(start_addr.align_up(PAGE_SIZE));
        let end_frame = start_frame + descriptor.page_count;
        if self.next_frame < end_frame {
            let ret = self.next_frame;
            self.next_frame += 1;
            Some(ret)
        } else {
            None
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for UefiFrameAllocator<'_> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        if let Some(current_descriptor) = self.current_descriptor {
            match self.allocate_frame_from_descriptor(current_descriptor) {
                Some(frame) => return Some(frame),
                None => {
                    self.current_descriptor = None;
                }
            }
        }

        // find next suitable descriptor
        while let Some(descriptor) = self.memory_map.next() {
            if descriptor.ty != MemoryType::CONVENTIONAL {
                continue;
            }
            if let Some(frame) = self.allocate_frame_from_descriptor(descriptor) {
                self.current_descriptor = Some(descriptor);
                return Some(frame);
            }
        }

        None
    }
}
