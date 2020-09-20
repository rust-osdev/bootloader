use crate::binary::legacy_memory_region::{LegacyFrameAllocator, LegacyMemoryRegion};
use crate::boot_info::{BootInfo, FrameBuffer, FrameBufferInfo};
use crate::memory_map::MemoryRegion;
use core::{
    mem::{self, MaybeUninit},
    slice,
};
use level_4_entries::UsedLevel4Entries;
use parsed_config::CONFIG;
use usize_conversions::FromUsize;
use x86_64::{
    registers,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame, Size2MiB,
        Size4KiB,
    },
    PhysAddr, VirtAddr,
};

#[cfg(feature = "bios_bin")]
pub mod bios;
#[cfg(feature = "uefi_bin")]
pub mod uefi;

pub mod legacy_memory_region;
pub mod level_4_entries;
pub mod load_kernel;
pub mod logger;

// Contains the parsed configuration table from the kernel's Cargo.toml.
//
// The layout of the file is the following:
//
// ```
// mod parsed_config {
//    pub const CONFIG: Config = Config { … };
// }
// ```
//
// The module file is created by the build script.
include!(concat!(env!("OUT_DIR"), "/bootloader_config.rs"));

const PAGE_SIZE: u64 = 4096;

pub fn init_logger(framebuffer: &'static mut [u8], info: FrameBufferInfo) {
    let logger = logger::LOGGER.get_or_init(move || logger::LockedLogger::new(framebuffer, info));
    log::set_logger(logger).expect("logger already set");
    log::set_max_level(log::LevelFilter::Trace);
}

#[derive(Debug, Copy, Clone)]
pub struct SystemInfo {
    pub framebuffer_addr: PhysAddr,
    pub framebuffer_info: FrameBufferInfo,
    pub rsdp_addr: Option<PhysAddr>,
}

pub fn load_and_switch_to_kernel<I, D>(
    kernel_bytes: &[u8],
    mut frame_allocator: LegacyFrameAllocator<I, D>,
    mut page_tables: PageTables,
    system_info: SystemInfo,
) -> !
where
    I: ExactSizeIterator<Item = D> + Clone,
    D: LegacyMemoryRegion,
{
    let mut mappings = set_up_mappings(
        kernel_bytes,
        &mut frame_allocator,
        &mut page_tables.kernel,
        system_info.framebuffer_addr,
        system_info.framebuffer_info.byte_len,
    );
    let (boot_info, two_frames) = create_boot_info(
        frame_allocator,
        &mut page_tables,
        &mut mappings,
        system_info,
    );
    switch_to_kernel(page_tables, mappings, boot_info, two_frames);
}

/// Sets up mappings for a kernel stack and the framebuffer
pub fn set_up_mappings<I, D>(
    kernel_bytes: &[u8],
    frame_allocator: &mut LegacyFrameAllocator<I, D>,
    kernel_page_table: &mut OffsetPageTable,
    framebuffer_addr: PhysAddr,
    framebuffer_size: usize,
) -> Mappings
where
    I: ExactSizeIterator<Item = D> + Clone,
    D: LegacyMemoryRegion,
{
    // Enable support for the no-execute bit in page tables.
    enable_nxe_bit();

    let (entry_point, mut used_entries) =
        load_kernel::load_kernel(kernel_bytes, kernel_page_table, frame_allocator)
            .expect("no entry point");
    log::info!("Entry point at: {:#x}", entry_point.as_u64());

    // create a stack
    let stack_start_addr = kernel_stack_start_location(&mut used_entries);
    let stack_start: Page = Page::containing_address(stack_start_addr);
    let stack_end = {
        let end_addr = stack_start_addr + CONFIG.kernel_stack_size.unwrap_or(20 * PAGE_SIZE);
        Page::containing_address(end_addr - 1u64)
    };
    for page in Page::range_inclusive(stack_start, stack_end) {
        let frame = frame_allocator
            .allocate_frame()
            .expect("frame allocation failed when mapping a kernel stack");
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { kernel_page_table.map_to(page, frame, flags, frame_allocator) }
            .unwrap()
            .flush();
    }

    // map framebuffer
    let framebuffer_virt_addr = if CONFIG.map_framebuffer {
        log::info!("Map framebuffer");

        let framebuffer_start_frame: PhysFrame = PhysFrame::containing_address(framebuffer_addr);
        let framebuffer_end_frame =
            PhysFrame::containing_address(framebuffer_addr + framebuffer_size - 1u64);
        let start_page = Page::containing_address(frame_buffer_location(&mut used_entries));
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
        Some(framebuffer_virt_addr)
    } else {
        None
    };

    let physical_memory_offset = if CONFIG.map_physical_memory {
        log::info!("Map physical memory");
        let offset = CONFIG
            .physical_memory_offset
            .map(VirtAddr::new)
            .unwrap_or_else(|| used_entries.get_free_address());

        let start_frame = PhysFrame::containing_address(PhysAddr::new(0));
        let max_phys = frame_allocator.max_phys_addr();
        let end_frame: PhysFrame<Size2MiB> = PhysFrame::containing_address(max_phys - 1u64);
        for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
            let page = Page::containing_address(offset + frame.start_address().as_u64());
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
            unsafe { kernel_page_table.map_to(page, frame, flags, frame_allocator) }
                .unwrap()
                .ignore();
        }

        Some(offset)
    } else {
        None
    };

    Mappings {
        framebuffer: framebuffer_virt_addr,
        entry_point,
        stack_end,
        used_entries,
        physical_memory_offset,
    }
}

pub struct Mappings {
    pub entry_point: VirtAddr,
    pub stack_end: Page,
    pub used_entries: UsedLevel4Entries,
    pub framebuffer: Option<VirtAddr>,
    pub physical_memory_offset: Option<VirtAddr>,
}

/// Allocates and initializes the boot info struct and the memory map
pub fn create_boot_info<I, D>(
    mut frame_allocator: LegacyFrameAllocator<I, D>,
    page_tables: &mut PageTables,
    mappings: &mut Mappings,
    system_info: SystemInfo,
) -> (&'static mut BootInfo, TwoFrames)
where
    I: ExactSizeIterator<Item = D> + Clone,
    D: LegacyMemoryRegion,
{
    log::info!("Allocate bootinfo");

    // allocate and map space for the boot info
    let (boot_info, memory_regions) = {
        let boot_info_addr = boot_info_location(&mut mappings.used_entries);
        let boot_info_end = boot_info_addr + mem::size_of::<BootInfo>();
        let memory_map_regions_addr =
            boot_info_end.align_up(u64::from_usize(mem::align_of::<MemoryRegion>()));
        let regions = frame_allocator.len() + 1; // one region might be split into used/unused
        let memory_map_regions_end =
            memory_map_regions_addr + regions * mem::size_of::<MemoryRegion>();

        let start_page = Page::containing_address(boot_info_addr);
        let end_page = Page::containing_address(memory_map_regions_end - 1u64);
        for page in Page::range_inclusive(start_page, end_page) {
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
            let frame = frame_allocator
                .allocate_frame()
                .expect("frame allocation for boot info failed");
            unsafe {
                page_tables
                    .kernel
                    .map_to(page, frame, flags, &mut frame_allocator)
            }
            .unwrap()
            .flush();
            // we need to be able to access it too
            unsafe {
                page_tables
                    .bootloader
                    .map_to(page, frame, flags, &mut frame_allocator)
            }
            .unwrap()
            .flush();
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
        framebuffer: mappings.framebuffer.map(|addr| FrameBuffer {
            buffer_start: addr.as_u64(),
            buffer_byte_len: system_info.framebuffer_info.byte_len,
            info: system_info.framebuffer_info,
        }),
        physical_memory_offset: mappings.physical_memory_offset.map(VirtAddr::as_u64),
        rsdp_addr: system_info.rsdp_addr.map(|addr| addr.as_u64()),
        _non_exhaustive: (),
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

fn boot_info_location(used_entries: &mut UsedLevel4Entries) -> VirtAddr {
    CONFIG
        .boot_info_address
        .map(VirtAddr::new)
        .unwrap_or_else(|| used_entries.get_free_address())
}

fn frame_buffer_location(used_entries: &mut UsedLevel4Entries) -> VirtAddr {
    CONFIG
        .framebuffer_address
        .map(VirtAddr::new)
        .unwrap_or_else(|| used_entries.get_free_address())
}

fn kernel_stack_start_location(used_entries: &mut UsedLevel4Entries) -> VirtAddr {
    CONFIG
        .kernel_stack_address
        .map(VirtAddr::new)
        .unwrap_or_else(|| used_entries.get_free_address())
}

fn enable_nxe_bit() {
    use x86_64::registers::control::{Efer, EferFlags};
    unsafe { Efer::update(|efer| *efer |= EferFlags::NO_EXECUTE_ENABLE) }
}
