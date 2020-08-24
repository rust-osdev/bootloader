use crate::binary::legacy_memory_region::{LegacyFrameAllocator, LegacyMemoryRegion};
use crate::boot_info::{BootInfo, FrameBufferInfo};
use crate::memory_map::MemoryRegion;
use core::{
    mem::{self, MaybeUninit},
    slice,
};
use usize_conversions::FromUsize;
use x86_64::{
    registers,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

#[cfg(feature = "bios_bin")]
pub mod bios;
#[cfg(feature = "uefi_bin")]
pub mod uefi;

pub mod legacy_memory_region;
pub mod load_kernel;
pub mod logger;

pub fn init_logger(framebuffer: &'static mut [u8], info: logger::FrameBufferInfo) {
    let logger = logger::LOGGER.get_or_init(move || logger::LockedLogger::new(framebuffer, info));
    log::set_logger(logger).expect("logger already set");
    log::set_max_level(log::LevelFilter::Trace);
}

pub fn load_and_switch_to_kernel<I, D>(
    kernel_bytes: &[u8],
    mut frame_allocator: LegacyFrameAllocator<I, D>,
    mut page_tables: PageTables,
    framebuffer_addr: PhysAddr,
    framebuffer_size: usize,
) -> !
where
    I: ExactSizeIterator<Item = D> + Clone,
    D: LegacyMemoryRegion,
{
    let mappings = set_up_mappings(
        kernel_bytes,
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

/// Sets up mappings for a kernel stack and the framebuffer
pub fn set_up_mappings(
    kernel_bytes: &[u8],
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    kernel_page_table: &mut OffsetPageTable,
    framebuffer_addr: PhysAddr,
    framebuffer_size: usize,
) -> Mappings {
    let entry_point = load_kernel::load_kernel(kernel_bytes, kernel_page_table, frame_allocator)
        .expect("no entry point");
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

pub struct Mappings {
    pub entry_point: VirtAddr,
    pub framebuffer: VirtAddr,
    pub stack_end: Page,
}

/// Allocates and initializes the boot info struct and the memory map
pub fn create_boot_info<I, D>(
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
pub fn switch_to_kernel(
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

pub struct PageTables {
    pub bootloader: OffsetPageTable<'static>,
    pub kernel: OffsetPageTable<'static>,
    pub kernel_level_4_frame: PhysFrame,
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

pub struct Addresses {
    page_table: PhysFrame,
    stack_top: VirtAddr,
    entry_point: VirtAddr,
    boot_info: &'static mut crate::boot_info::BootInfo,
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
