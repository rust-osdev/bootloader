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

use bootloader::binary::{
    legacy_memory_region::{LegacyMemoryRegion, LegacyFrameAllocator},
    uefi::load_kernel,
};
use bootloader::boot_info_uefi::{BootInfo, FrameBufferInfo};
use bootloader::memory_map::MemoryRegion;
use core::{
    mem::{self, MaybeUninit},
    panic::PanicInfo,
    slice,
};
use uefi::{
    prelude::{entry, Boot, Handle, ResultExt, Status, SystemTable},
    proto::console::gop::{GraphicsOutput, PixelFormat},
    table::boot::{MemoryDescriptor, MemoryType},
};
use usize_conversions::FromUsize;
use x86_64::{
    registers,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame,
        Size4KiB,
    },
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
    let mut page_tables = create_page_tables(&mut frame_allocator);
    let mappings = set_up_mappings(
        &mut frame_allocator,
        &mut page_tables.kernel,
        framebuffer_addr,
        framebuffer_size,
    );
    let (boot_info, two_frames) = create_boot_info(
        frame_allocator,
        &mut page_tables,
        mappings.framebuffer,
        framebuffer_size,
    );
    switch_to_kernel(page_tables, mappings, boot_info, two_frames);
}

/// Creates page table abstraction types for both the bootloader and kernel page tables.
fn create_page_tables(frame_allocator: &mut impl FrameAllocator<Size4KiB>) -> PageTables {
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

    PageTables {
        bootloader: bootloader_page_table,
        kernel: kernel_page_table,
        kernel_level_4_frame,
    }
}

struct PageTables {
    bootloader: OffsetPageTable<'static>,
    kernel: OffsetPageTable<'static>,
    kernel_level_4_frame: PhysFrame,
}

/// Sets up mappings for a kernel stack and the framebuffer
fn set_up_mappings(
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    kernel_page_table: &mut OffsetPageTable,
    framebuffer_addr: PhysAddr,
    framebuffer_size: usize,
) -> Mappings {
    let entry_point = load_kernel(&KERNEL.0, kernel_page_table, frame_allocator);
    log::info!("Entry point at: {:#x}", entry_point.as_u64());

    // create a stack
    let stack_start: Page = kernel_stack_start_location();
    let stack_end = stack_start + 20;
    for page in Page::range(stack_start, stack_end) {
        let frame = frame_allocator
            .allocate_frame()
            .expect("frame allocation failed when mapping a kernel stack");
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { kernel_page_table.map_to(page, frame, flags, frame_allocator) }
            .unwrap()
            .flush();
    }

    log::info!("Map framebuffer");

    // map framebuffer
    let framebuffer_start_frame: PhysFrame = PhysFrame::containing_address(framebuffer_addr);
    let framebuffer_end_frame =
        PhysFrame::containing_address(framebuffer_addr + framebuffer_size - 1u64);
    let start_page = frame_buffer_location();
    for (i, frame) in
        PhysFrame::range_inclusive(framebuffer_start_frame, framebuffer_end_frame).enumerate()
    {
        let page = start_page + u64::from_usize(i);
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { kernel_page_table.map_to(page, frame, flags, frame_allocator) }
            .unwrap()
            .flush();
    }
    let framebuffer_virt_addr = start_page.start_address();

    Mappings {
        framebuffer: framebuffer_virt_addr,
        entry_point,
        stack_end,
    }
}

struct Mappings {
    entry_point: VirtAddr,
    framebuffer: VirtAddr,
    stack_end: Page,
}

/// Allocates and initializes the boot info struct and the memory map
fn create_boot_info<I, D>(
    mut frame_allocator: LegacyFrameAllocator<I, D>,
    page_tables: &mut PageTables,
    framebuffer_virt_addr: VirtAddr,
    framebuffer_size: usize,
) -> (&'static mut BootInfo, TwoFrames)
where
    I: ExactSizeIterator<Item = D> + Clone,
    D: LegacyMemoryRegion,
{
    log::info!("Allocate bootinfo");

    // allocate and map space for the boot info
    let (boot_info, memory_regions) = {
        let boot_info_addr = boot_info_location();
        let boot_info_end = boot_info_addr + mem::size_of::<BootInfo>();
        let memory_map_regions_addr =
            boot_info_end.align_up(u64::from_usize(mem::align_of::<MemoryRegion>()));
        let regions = frame_allocator.len() + 1; // one region might be split into used/unused
        let memory_map_regions_end =
            memory_map_regions_addr + regions * mem::size_of::<MemoryRegion>();

        let start_page = Page::containing_address(boot_info_addr);
        let end_page = Page::containing_address(memory_map_regions_end - 1u64);
        for page in Page::range_inclusive(start_page, end_page) {
            log::info!("Mapping page {:?}", page);
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
            let frame = frame_allocator
                .allocate_frame()
                .expect("frame allocation for boot info failed");
            log::info!("1 {:?}", page);
            unsafe {
                page_tables
                    .kernel
                    .map_to(page, frame, flags, &mut frame_allocator)
            }
            .unwrap()
            .flush();
            log::info!("2 {:?}", page);
            // we need to be able to access it too
            unsafe {
                page_tables
                    .bootloader
                    .map_to(page, frame, flags, &mut frame_allocator)
            }
            .unwrap()
            .flush();
            log::info!("Finished mapping page {:?}", page);
        }

        let boot_info: &'static mut MaybeUninit<BootInfo> =
            unsafe { &mut *boot_info_addr.as_mut_ptr() };
        let memory_regions: &'static mut [MaybeUninit<MemoryRegion>] =
            unsafe { slice::from_raw_parts_mut(memory_map_regions_addr.as_mut_ptr(), regions) };
        (boot_info, memory_regions)
    };

    // reserve two unused frames for context switch
    let two_frames = TwoFrames::new(&mut frame_allocator);

    log::info!("Create Memory Map");

    // build memory map
    let memory_regions = frame_allocator.construct_memory_map(memory_regions);

    log::info!("Create bootinfo");

    // create boot info
    let boot_info = boot_info.write(BootInfo {
        memory_regions,
        framebuffer: FrameBufferInfo {
            start_addr: framebuffer_virt_addr.as_u64(),
            len: framebuffer_size,
        },
    });

    (boot_info, two_frames)
}

/// Switches to the kernel address space and jumps to the kernel entry point.
fn switch_to_kernel(
    page_tables: PageTables,
    mappings: Mappings,
    boot_info: &'static mut BootInfo,
    two_frames: TwoFrames,
) -> ! {
    let PageTables {
        kernel_level_4_frame,
        kernel: kernel_page_table,
        ..
    } = page_tables;
    let addresses = Addresses {
        page_table: kernel_level_4_frame,
        stack_top: mappings.stack_end.start_address(),
        entry_point: mappings.entry_point,
        boot_info,
    };

    log::info!(
        "Jumping to kernel entry point at {:?}",
        addresses.entry_point
    );
    unsafe {
        context_switch(addresses, kernel_page_table, two_frames);
    }
}

/// Performs the actual context switch
///
/// This function should stay small because it needs to be identity-mapped.
unsafe fn context_switch(
    addresses: Addresses,
    mut kernel_page_table: OffsetPageTable,
    mut frame_allocator: impl FrameAllocator<Size4KiB>,
) -> ! {
    // identity-map current and next frame, so that we don't get an immediate pagefault
    // after switching the active page table
    let current_addr = PhysAddr::new(registers::read_rip());
    let current_frame: PhysFrame = PhysFrame::containing_address(current_addr);
    for frame in PhysFrame::range_inclusive(current_frame, current_frame + 1) {
        unsafe {
            kernel_page_table.identity_map(frame, PageTableFlags::PRESENT, &mut frame_allocator)
        }
        .unwrap()
        .flush();
    }

    // we don't need the kernel page table anymore
    mem::drop(kernel_page_table);

    // do the context switch
    unsafe {
        asm!(
            "mov cr3, {}; mov rsp, {}; push 0; jmp {}",
            in(reg) addresses.page_table.start_address().as_u64(),
            in(reg) addresses.stack_top.as_u64(),
            in(reg) addresses.entry_point.as_u64(),
            in("rdi") addresses.boot_info as *const _ as usize,
        );
    }
    unreachable!();
}

struct Addresses {
    page_table: PhysFrame,
    stack_top: VirtAddr,
    entry_point: VirtAddr,
    boot_info: &'static mut bootloader::boot_info_uefi::BootInfo,
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
    let info = bootloader::binary::uefi::FrameBufferInfo {
        horizontal_resolution: mode_info.resolution().0,
        vertical_resolution: mode_info.resolution().1,
        pixel_format: match mode_info.pixel_format() {
            PixelFormat::RGB => bootloader::binary::uefi::PixelFormat::BGR,
            PixelFormat::BGR => bootloader::binary::uefi::PixelFormat::BGR,
            PixelFormat::Bitmask | PixelFormat::BltOnly => {
                panic!("Bitmask and BltOnly framebuffers are not supported")
            }
        },
        stride: mode_info.stride(),
    };

    bootloader::binary::uefi::init_logger(slice, info);

    (
        PhysAddr::new(framebuffer.as_mut_ptr() as u64),
        framebuffer.size(),
    )
}

pub struct TwoFrames {
    frames: [Option<PhysFrame>; 2],
}

impl TwoFrames {
    pub fn new(frame_allocator: &mut impl FrameAllocator<Size4KiB>) -> Self {
        TwoFrames {
            frames: [
                Some(frame_allocator.allocate_frame().unwrap()),
                Some(frame_allocator.allocate_frame().unwrap()),
            ],
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for TwoFrames {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        self.frames.iter_mut().find_map(|f| f.take())
    }
}

fn boot_info_location() -> VirtAddr {
    VirtAddr::new(0x_0000_00bb_bbbb_0000)
}

fn frame_buffer_location() -> Page {
    Page::containing_address(VirtAddr::new(0x_0000_00cc_cccc_0000))
}

fn kernel_stack_start_location() -> Page {
    Page::containing_address(VirtAddr::new(0x_0000_0fff_0000_0000))
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe { bootloader::binary::uefi::logger::LOGGER.get().map(|l| l.force_unlock()) };
    log::error!("{}", info);
    loop {
        unsafe { asm!("cli; hlt") };
    }
}
