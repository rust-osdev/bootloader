use crate::binary::legacy_memory_region::{LegacyFrameAllocator, LegacyMemoryRegion};
use bootloader_api::{
    config::Mapping,
    info::{FrameBuffer, FrameBufferInfo, MemoryRegion, TlsTemplate},
    BootInfo, BootloaderConfig,
};
use core::{
    arch::asm,
    mem::{self, MaybeUninit},
    slice,
};
use level_4_entries::UsedLevel4Entries;
use usize_conversions::FromUsize;
use x86_64::{
    structures::paging::{
        page_table::PageTableLevel, FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags,
        PageTableIndex, PhysFrame, Size2MiB,
    },
    PhysAddr, VirtAddr,
};
use xmas_elf::ElfFile;

/// Provides BIOS-specific types and trait implementations.
#[cfg(feature = "bios_bin")]
pub mod bios;
/// Provides UEFI-specific trait implementations.
#[cfg(feature = "uefi_bin")]
mod uefi;

mod gdt;
/// Provides a frame allocator based on a BIOS or UEFI memory map.
pub mod legacy_memory_region;
/// Provides a type to keep track of used entries in a level 4 page table.
pub mod level_4_entries;
/// Implements a loader for the kernel ELF binary.
pub mod load_kernel;
/// Provides a logger type that logs output as text to pixel-based framebuffers.
pub mod logger;

const PAGE_SIZE: u64 = 4096;

/// Initialize a text-based logger using the given pixel-based framebuffer as output.  
pub fn init_logger(framebuffer: &'static mut [u8], info: FrameBufferInfo) {
    let logger = logger::LOGGER.get_or_init(move || logger::LockedLogger::new(framebuffer, info));
    log::set_logger(logger).expect("logger already set");
    log::set_max_level(log::LevelFilter::Trace);
    log::info!("Framebuffer info: {:?}", info);
}

/// Required system information that should be queried from the BIOS or UEFI firmware.
#[derive(Debug, Copy, Clone)]
pub struct SystemInfo {
    /// Start address of the pixel-based framebuffer.
    pub framebuffer_addr: PhysAddr,
    /// Information about the framebuffer, including layout and pixel format.
    pub framebuffer_info: FrameBufferInfo,
    /// Address of the _Root System Description Pointer_ structure of the ACPI standard.
    pub rsdp_addr: Option<PhysAddr>,
}

pub struct Kernel<'a> {
    elf: ElfFile<'a>,
    config: BootloaderConfig,
}

/// Loads the kernel ELF executable into memory and switches to it.
///
/// This function is a convenience function that first calls [`set_up_mappings`], then
/// [`create_boot_info`], and finally [`switch_to_kernel`]. The given arguments are passed
/// directly to these functions, so see their docs for more info.
pub fn load_and_switch_to_kernel<I, D>(
    kernel: Kernel,
    mut frame_allocator: LegacyFrameAllocator<I, D>,
    mut page_tables: PageTables,
    system_info: SystemInfo,
) -> !
where
    I: ExactSizeIterator<Item = D> + Clone,
    D: LegacyMemoryRegion,
{
    let config = kernel.config;
    let mut mappings = set_up_mappings(
        kernel,
        &mut frame_allocator,
        &mut page_tables,
        system_info.framebuffer_addr,
        system_info.framebuffer_info.byte_len,
    );
    let boot_info = create_boot_info(
        &config,
        frame_allocator,
        &mut page_tables,
        &mut mappings,
        system_info,
    );
    switch_to_kernel(page_tables, mappings, boot_info);
}

/// Sets up mappings for a kernel stack and the framebuffer.
///
/// The `kernel_bytes` slice should contain the raw bytes of the kernel ELF executable. The
/// `frame_allocator` argument should be created from the memory map. The `page_tables`
/// argument should point to the bootloader and kernel page tables. The function tries to parse
/// the ELF file and create all specified mappings in the kernel-level page table.
///
/// The `framebuffer_addr` and `framebuffer_size` fields should be set to the start address and
/// byte length the pixel-based framebuffer. These arguments are required because the functions
/// maps this framebuffer in the kernel-level page table, unless the `map_framebuffer` config
/// option is disabled.
///
/// This function reacts to unexpected situations (e.g. invalid kernel ELF file) with a panic, so
/// errors are not recoverable.
pub fn set_up_mappings<I, D>(
    kernel: Kernel,
    frame_allocator: &mut LegacyFrameAllocator<I, D>,
    page_tables: &mut PageTables,
    framebuffer_addr: PhysAddr,
    framebuffer_size: usize,
) -> Mappings
where
    I: ExactSizeIterator<Item = D> + Clone,
    D: LegacyMemoryRegion,
{
    let kernel_page_table = &mut page_tables.kernel;

    // Enable support for the no-execute bit in page tables.
    enable_nxe_bit();
    // Make the kernel respect the write-protection bits even when in ring 0 by default
    enable_write_protect_bit();

    let config = kernel.config;
    let (entry_point, tls_template, mut used_entries) =
        load_kernel::load_kernel(kernel, kernel_page_table, frame_allocator)
            .expect("no entry point");
    log::info!("Entry point at: {:#x}", entry_point.as_u64());

    // create a stack
    let stack_start_addr = mapping_addr(config.mappings.kernel_stack, &mut used_entries);
    let stack_start: Page = Page::containing_address(stack_start_addr);
    let stack_end = {
        let end_addr = stack_start_addr + config.kernel_stack_size;
        Page::containing_address(end_addr - 1u64)
    };
    for page in Page::range_inclusive(stack_start, stack_end) {
        let frame = frame_allocator
            .allocate_frame()
            .expect("frame allocation failed when mapping a kernel stack");
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        match unsafe { kernel_page_table.map_to(page, frame, flags, frame_allocator) } {
            Ok(tlb) => tlb.flush(),
            Err(err) => panic!("failed to map page {:?}: {:?}", page, err),
        }
    }

    // identity-map context switch function, so that we don't get an immediate pagefault
    // after switching the active page table
    let context_switch_function = PhysAddr::new(context_switch as *const () as u64);
    let context_switch_function_start_frame: PhysFrame =
        PhysFrame::containing_address(context_switch_function);
    for frame in PhysFrame::range_inclusive(
        context_switch_function_start_frame,
        context_switch_function_start_frame + 1,
    ) {
        match unsafe {
            kernel_page_table.identity_map(frame, PageTableFlags::PRESENT, frame_allocator)
        } {
            Ok(tlb) => tlb.flush(),
            Err(err) => panic!("failed to identity map frame {:?}: {:?}", frame, err),
        }
    }

    // create, load, and identity-map GDT (required for working `iretq`)
    let gdt_frame = frame_allocator
        .allocate_frame()
        .expect("failed to allocate GDT frame");
    gdt::create_and_load(gdt_frame);
    match unsafe {
        kernel_page_table.identity_map(gdt_frame, PageTableFlags::PRESENT, frame_allocator)
    } {
        Ok(tlb) => tlb.flush(),
        Err(err) => panic!("failed to identity map frame {:?}: {:?}", gdt_frame, err),
    }

    // map framebuffer
    let framebuffer_virt_addr = {
        log::info!("Map framebuffer");

        let framebuffer_start_frame: PhysFrame = PhysFrame::containing_address(framebuffer_addr);
        let framebuffer_end_frame =
            PhysFrame::containing_address(framebuffer_addr + framebuffer_size - 1u64);
        let start_page =
            Page::containing_address(mapping_addr(config.mappings.framebuffer, &mut used_entries));
        for (i, frame) in
            PhysFrame::range_inclusive(framebuffer_start_frame, framebuffer_end_frame).enumerate()
        {
            let page = start_page + u64::from_usize(i);
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
            match unsafe { kernel_page_table.map_to(page, frame, flags, frame_allocator) } {
                Ok(tlb) => tlb.flush(),
                Err(err) => panic!(
                    "failed to map page {:?} to frame {:?}: {:?}",
                    page, frame, err
                ),
            }
        }
        let framebuffer_virt_addr = start_page.start_address();
        Some(framebuffer_virt_addr)
    };

    let physical_memory_offset = if let Some(mapping) = config.mappings.physical_memory {
        log::info!("Map physical memory");
        let offset = mapping_addr(mapping, &mut used_entries);

        let start_frame = PhysFrame::containing_address(PhysAddr::new(0));
        let max_phys = frame_allocator.max_phys_addr();
        let end_frame: PhysFrame<Size2MiB> = PhysFrame::containing_address(max_phys - 1u64);
        for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
            let page = Page::containing_address(offset + frame.start_address().as_u64());
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
            match unsafe { kernel_page_table.map_to(page, frame, flags, frame_allocator) } {
                Ok(tlb) => tlb.ignore(),
                Err(err) => panic!(
                    "failed to map page {:?} to frame {:?}: {:?}",
                    page, frame, err
                ),
            };
        }

        Some(offset)
    } else {
        None
    };

    let recursive_index = if let Some(mapping) = config.mappings.page_table_recursive {
        log::info!("Map page table recursively");
        let offset = mapping_addr(mapping, &mut used_entries);
        let table_level = PageTableLevel::Four;
        if !offset.is_aligned(table_level.entry_address_space_alignment()) {
            panic!(
                "Offset for recursive mapping must be properly aligned (must be \
                a multiple of {:#x})",
                table_level.entry_address_space_alignment()
            );
        }

        let index = offset.p4_index();

        let entry = &mut kernel_page_table.level_4_table()[index];
        if !entry.is_unused() {
            panic!(
                "Could not set up recursive mapping: index {} already in use",
                u16::from(index)
            );
        }
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        entry.set_frame(page_tables.kernel_level_4_frame, flags);

        Some(index)
    } else {
        None
    };

    Mappings {
        framebuffer: framebuffer_virt_addr,
        entry_point,
        stack_end,
        used_entries,
        physical_memory_offset,
        recursive_index,
        tls_template,
    }
}

/// Contains the addresses of all memory mappings set up by [`set_up_mappings`].
pub struct Mappings {
    /// The entry point address of the kernel.
    pub entry_point: VirtAddr,
    /// The stack end page of the kernel.
    pub stack_end: Page,
    /// Keeps track of used entries in the level 4 page table, useful for finding a free
    /// virtual memory when needed.
    pub used_entries: UsedLevel4Entries,
    /// The start address of the framebuffer, if any.
    pub framebuffer: Option<VirtAddr>,
    /// The start address of the physical memory mapping, if enabled.
    pub physical_memory_offset: Option<VirtAddr>,
    /// The level 4 page table index of the recursive mapping, if enabled.
    pub recursive_index: Option<PageTableIndex>,
    /// The thread local storage template of the kernel executable, if it contains one.
    pub tls_template: Option<TlsTemplate>,
}

/// Allocates and initializes the boot info struct and the memory map.
///
/// The boot info and memory map are mapped to both the kernel and bootloader
/// address space at the same address. This makes it possible to return a Rust
/// reference that is valid in both address spaces. The necessary physical frames
/// are taken from the given `frame_allocator`.
pub fn create_boot_info<I, D>(
    config: &BootloaderConfig,
    mut frame_allocator: LegacyFrameAllocator<I, D>,
    page_tables: &mut PageTables,
    mappings: &mut Mappings,
    system_info: SystemInfo,
) -> &'static mut BootInfo
where
    I: ExactSizeIterator<Item = D> + Clone,
    D: LegacyMemoryRegion,
{
    log::info!("Allocate bootinfo");

    // allocate and map space for the boot info
    let (boot_info, memory_regions) = {
        let boot_info_addr = mapping_addr(config.mappings.boot_info, &mut mappings.used_entries);
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
            match unsafe {
                page_tables
                    .kernel
                    .map_to(page, frame, flags, &mut frame_allocator)
            } {
                Ok(tlb) => tlb.flush(),
                Err(err) => panic!("failed to map page {:?}: {:?}", page, err),
            }
            // we need to be able to access it too
            match unsafe {
                page_tables
                    .bootloader
                    .map_to(page, frame, flags, &mut frame_allocator)
            } {
                Ok(tlb) => tlb.flush(),
                Err(err) => panic!("failed to map page {:?}: {:?}", page, err),
            }
        }

        let boot_info: &'static mut MaybeUninit<BootInfo> =
            unsafe { &mut *boot_info_addr.as_mut_ptr() };
        let memory_regions: &'static mut [MaybeUninit<MemoryRegion>] =
            unsafe { slice::from_raw_parts_mut(memory_map_regions_addr.as_mut_ptr(), regions) };
        (boot_info, memory_regions)
    };

    log::info!("Create Memory Map");

    // build memory map
    let memory_regions = frame_allocator.construct_memory_map(memory_regions);

    log::info!("Create bootinfo");

    // create boot info
    let boot_info = boot_info.write({
        let mut info = BootInfo::new(memory_regions.into());
        info.framebuffer = mappings
            .framebuffer
            .map(|addr| {
                FrameBuffer::new(
                    addr.as_u64(),
                    system_info.framebuffer_info.byte_len,
                    system_info.framebuffer_info,
                )
            })
            .into();
        info.physical_memory_offset = mappings.physical_memory_offset.map(VirtAddr::as_u64).into();
        info.recursive_index = mappings.recursive_index.map(Into::into).into();
        info.rsdp_addr = system_info.rsdp_addr.map(|addr| addr.as_u64()).into();
        info.tls_template = mappings.tls_template.into();
        info
    });

    boot_info
}

/// Switches to the kernel address space and jumps to the kernel entry point.
pub fn switch_to_kernel(
    page_tables: PageTables,
    mappings: Mappings,
    boot_info: &'static mut BootInfo,
) -> ! {
    let PageTables {
        kernel_level_4_frame,
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
        context_switch(addresses);
    }
}

/// Provides access to the page tables of the bootloader and kernel address space.
pub struct PageTables {
    /// Provides access to the page tables of the bootloader address space.
    pub bootloader: OffsetPageTable<'static>,
    /// Provides access to the page tables of the kernel address space (not active).
    pub kernel: OffsetPageTable<'static>,
    /// The physical frame where the level 4 page table of the kernel address space is stored.
    ///
    /// Must be the page table that the `kernel` field of this struct refers to.
    ///
    /// This frame is loaded into the `CR3` register on the final context switch to the kernel.  
    pub kernel_level_4_frame: PhysFrame,
}

/// Performs the actual context switch.
unsafe fn context_switch(addresses: Addresses) -> ! {
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

/// Memory addresses required for the context switch.
struct Addresses {
    page_table: PhysFrame,
    stack_top: VirtAddr,
    entry_point: VirtAddr,
    boot_info: &'static mut BootInfo,
}

fn mapping_addr(mapping: Mapping, used_entries: &mut UsedLevel4Entries) -> VirtAddr {
    match mapping {
        Mapping::FixedAddress(addr) => VirtAddr::new(addr),
        Mapping::Dynamic => used_entries.get_free_address(),
    }
}

fn enable_nxe_bit() {
    use x86_64::registers::control::{Efer, EferFlags};
    unsafe { Efer::update(|efer| *efer |= EferFlags::NO_EXECUTE_ENABLE) }
}

fn enable_write_protect_bit() {
    use x86_64::registers::control::{Cr0, Cr0Flags};
    unsafe { Cr0::update(|cr0| *cr0 |= Cr0Flags::WRITE_PROTECT) };
}
