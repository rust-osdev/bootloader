#![no_std]
#![feature(step_trait)]
#![deny(unsafe_op_in_unsafe_fn)]

use crate::legacy_memory_region::{LegacyFrameAllocator, LegacyMemoryRegion};
use bootloader_api::{
    config::Mapping,
    info::{FrameBuffer, FrameBufferInfo, MemoryRegion, TlsTemplate},
    BootInfo, BootloaderConfig,
};
use core::{alloc::Layout, arch::asm, mem::MaybeUninit, slice};
use level_4_entries::UsedLevel4Entries;
use usize_conversions::{FromUsize, IntoUsize};
use x86_64::{
    structures::paging::{
        page_table::PageTableLevel, FrameAllocator, Mapper, OffsetPageTable, Page, PageSize,
        PageTable, PageTableFlags, PageTableIndex, PhysFrame, Size2MiB, Size4KiB,
    },
    PhysAddr, VirtAddr,
};
use xmas_elf::ElfFile;

/// Provides a function to gather entropy and build a RNG.
mod entropy;
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
    /// Information about the (still unmapped) framebuffer.
    pub framebuffer: Option<RawFrameBufferInfo>,
    /// Address of the _Root System Description Pointer_ structure of the ACPI standard.
    pub rsdp_addr: Option<PhysAddr>,
}

/// The physical address of the framebuffer and information about the framebuffer.
#[derive(Debug, Copy, Clone)]
pub struct RawFrameBufferInfo {
    /// Start address of the pixel-based framebuffer.
    pub addr: PhysAddr,
    /// Information about the framebuffer, including layout and pixel format.
    pub info: FrameBufferInfo,
}

pub struct Kernel<'a> {
    pub elf: ElfFile<'a>,
    pub config: BootloaderConfig,
    pub start_address: *const u8,
    pub len: usize,
}

impl<'a> Kernel<'a> {
    pub fn parse(kernel_slice: &'a [u8]) -> Self {
        let kernel_elf = ElfFile::new(kernel_slice).unwrap();
        let config = {
            let section = kernel_elf
                .find_section_by_name(".bootloader-config")
                .expect("bootloader config section not found; kernel must be compiled against bootloader_api");
            let raw = section.raw_data(&kernel_elf);
            BootloaderConfig::deserialize(raw)
                .expect("kernel was compiled with incompatible bootloader_api version")
        };
        Kernel {
            elf: kernel_elf,
            config,
            start_address: kernel_slice.as_ptr(),
            len: kernel_slice.len(),
        }
    }
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
        system_info.framebuffer.as_ref(),
        &config,
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
    framebuffer: Option<&RawFrameBufferInfo>,
    config: &BootloaderConfig,
) -> Mappings
where
    I: ExactSizeIterator<Item = D> + Clone,
    D: LegacyMemoryRegion,
{
    let bootloader_page_table = &mut page_tables.bootloader;
    let kernel_page_table = &mut page_tables.kernel;

    let mut used_entries = UsedLevel4Entries::new(
        frame_allocator.max_phys_addr(),
        frame_allocator.len(),
        framebuffer,
        config,
    );

    // Enable support for the no-execute bit in page tables.
    enable_nxe_bit();
    // Make the kernel respect the write-protection bits even when in ring 0 by default
    enable_write_protect_bit();

    let config = kernel.config;
    let kernel_slice_start = kernel.start_address as u64;
    let kernel_slice_len = u64::try_from(kernel.len).unwrap();

    let (entry_point, tls_template) = load_kernel::load_kernel(
        kernel,
        kernel_page_table,
        frame_allocator,
        &mut used_entries,
    )
    .expect("no entry point");
    log::info!("Entry point at: {:#x}", entry_point.as_u64());

    // create a stack
    let stack_start_addr = mapping_addr(
        config.mappings.kernel_stack,
        config.kernel_stack_size,
        16,
        &mut used_entries,
    );
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
    let framebuffer_virt_addr = if let Some(framebuffer) = framebuffer {
        log::info!("Map framebuffer");

        let framebuffer_start_frame: PhysFrame = PhysFrame::containing_address(framebuffer.addr);
        let framebuffer_end_frame =
            PhysFrame::containing_address(framebuffer.addr + framebuffer.info.byte_len - 1u64);
        let start_page = Page::from_start_address(mapping_addr(
            config.mappings.framebuffer,
            u64::from_usize(framebuffer.info.byte_len),
            Size4KiB::SIZE,
            &mut used_entries,
        ))
        .expect("the framebuffer address must be page aligned");
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
    } else {
        None
    };

    let physical_memory_offset = if let Some(mapping) = config.mappings.physical_memory {
        log::info!("Map physical memory");

        let start_frame = PhysFrame::containing_address(PhysAddr::new(0));
        let max_phys = frame_allocator.max_phys_addr();
        let end_frame: PhysFrame<Size2MiB> = PhysFrame::containing_address(max_phys - 1u64);

        let size = max_phys.as_u64();
        let alignment = Size2MiB::SIZE;
        let offset = mapping_addr(mapping, size, alignment, &mut used_entries);

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
        let index = match mapping {
            Mapping::Dynamic => used_entries.get_free_entries(1),
            Mapping::FixedAddress(offset) => {
                let offset = VirtAddr::new(offset);
                let table_level = PageTableLevel::Four;
                if !offset.is_aligned(table_level.entry_address_space_alignment()) {
                    panic!(
                        "Offset for recursive mapping must be properly aligned (must be \
                        a multiple of {:#x})",
                        table_level.entry_address_space_alignment()
                    );
                }

                offset.p4_index()
            }
        };

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

    // Setup memory for the context switch.
    // We set up two regions of memory:
    // 1. "context switch page" - This page contains only a single instruction
    //    to switch to the kernel's page table. It's placed right before the
    //    kernel's entrypoint, so that the last instruction the bootloader
    //    executes is the page table switch and we don't need to jump to the
    //    entrypoint.
    // 2. "trampoline" - The "context switch page" might overlap with the
    //    bootloader's memory, so we can't map it into the bootloader's address
    //    space. Instead we map a trampoline at an address of our choosing and
    //    jump to it instead. The trampoline will then switch to a new page
    //    table (context switch page table) that contains the "context switch
    //    page" and jump to it.

    let phys_offset = kernel_page_table.phys_offset();
    let translate_frame_to_virt = |frame: PhysFrame| phys_offset + frame.start_address().as_u64();

    // The switching the page table is a 3 byte instruction.
    // Check that subtraction 3 from the entrypoint won't jump the gap in the address space.
    if (0xffff_8000_0000_0000..=0xffff_8000_0000_0002).contains(&entry_point.as_u64()) {
        panic!("The kernel's entrypoint must not be located between 0xffff_8000_0000_0000 and 0xffff_8000_0000_0002");
    }
    // Determine the address where we should place the page table switch instruction.
    let entrypoint_page: Page = Page::containing_address(entry_point);
    let addr_just_before_entrypoint = entry_point.as_u64().wrapping_sub(3);
    let context_switch_addr = VirtAddr::new(addr_just_before_entrypoint);
    let context_switch_page: Page = Page::containing_address(context_switch_addr);

    // Choose the address for the trampoline. The address shouldn't overlap
    // with the bootloader's memory or the context switch page.
    let trampoline_page_candidate1: Page =
        Page::from_start_address(VirtAddr::new(0xffff_ffff_ffff_f000)).unwrap();
    let trampoline_page_candidate2: Page =
        Page::from_start_address(VirtAddr::new(0xffff_ffff_ffff_c000)).unwrap();
    let trampoline_page = if context_switch_page != trampoline_page_candidate1
        && entrypoint_page != trampoline_page_candidate1
    {
        trampoline_page_candidate1
    } else {
        trampoline_page_candidate2
    };

    // Prepare the trampoline.
    let trampoline_frame = frame_allocator
        .allocate_frame()
        .expect("Failed to allocate memory for trampoline");
    // Write two instructions to the trampoline:
    // 1. Load the context switch page table
    // 2. Jump to the context switch
    unsafe {
        let trampoline: *mut u8 = translate_frame_to_virt(trampoline_frame).as_mut_ptr();
        // mov cr3, rdx
        trampoline.add(0).write(0x0f);
        trampoline.add(1).write(0x22);
        trampoline.add(2).write(0xda);
        // jmp r13
        trampoline.add(3).write(0x41);
        trampoline.add(4).write(0xff);
        trampoline.add(5).write(0xe5);
    }

    // Write the instruction to switch to the final kernel page table to the context switch page.
    let context_switch_frame = frame_allocator
        .allocate_frame()
        .expect("Failed to allocate memory for context switch page");
    // mov cr3, rax
    let instruction_bytes = [0x0f, 0x22, 0xd8];
    let context_switch_ptr: *mut u8 = translate_frame_to_virt(context_switch_frame).as_mut_ptr();
    for (i, b) in instruction_bytes.into_iter().enumerate() {
        // We can let the offset wrap around because we map the frame twice
        // if the context switch is near a page boundary.
        let offset = (context_switch_addr.as_u64().into_usize()).wrapping_add(i) % 4096;

        unsafe {
            // Write the instruction byte.
            context_switch_ptr.add(offset).write(b);
        }
    }

    // Create a new page table for use during the context switch.
    let context_switch_page_table_frame = frame_allocator
        .allocate_frame()
        .expect("Failed to allocate frame for context switch page table");
    let context_switch_page_table: &mut PageTable = {
        let ptr: *mut PageTable =
            translate_frame_to_virt(context_switch_page_table_frame).as_mut_ptr();
        // create a new, empty page table
        unsafe {
            ptr.write(PageTable::new());
            &mut *ptr
        }
    };
    let mut context_switch_page_table =
        unsafe { OffsetPageTable::new(context_switch_page_table, phys_offset) };

    // Map the trampoline and the context switch.
    unsafe {
        // Map the trampoline page into both the bootloader's page table and
        // the context switch page table.
        bootloader_page_table
            .map_to(
                trampoline_page,
                trampoline_frame,
                PageTableFlags::PRESENT,
                frame_allocator,
            )
            .expect("Failed to map trampoline into main page table")
            .ignore();
        context_switch_page_table
            .map_to(
                trampoline_page,
                trampoline_frame,
                PageTableFlags::PRESENT,
                frame_allocator,
            )
            .expect("Failed to map trampoline into context switch page table")
            .ignore();

        // Map the context switch only into the context switch page table.
        context_switch_page_table
            .map_to(
                context_switch_page,
                context_switch_frame,
                PageTableFlags::PRESENT,
                frame_allocator,
            )
            .expect("Failed to map context switch into context switch page table")
            .ignore();

        // If the context switch is near a page boundary, map the entrypoint
        // page to the same frame in case the page table switch instruction
        // crosses a page boundary.
        if context_switch_page != entrypoint_page {
            context_switch_page_table
                .map_to(
                    entrypoint_page,
                    context_switch_frame,
                    PageTableFlags::PRESENT,
                    frame_allocator,
                )
                .expect("Failed to map context switch into context switch page table")
                .ignore();
        }
    }

    Mappings {
        framebuffer: framebuffer_virt_addr,
        entry_point,
        stack_end,
        used_entries,
        physical_memory_offset,
        recursive_index,
        tls_template,

        kernel_slice_start,
        kernel_slice_len,
        context_switch_trampoline: trampoline_page.start_address(),
        context_switch_page_table,
        context_switch_page_table_frame,
        context_switch_addr,
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

    /// Start address of the kernel slice allocation in memory.
    pub kernel_slice_start: u64,
    /// Size of the kernel slice allocation in memory.
    pub kernel_slice_len: u64,
    /// The address of the context switch trampoline in the bootloader's address space.
    pub context_switch_trampoline: VirtAddr,
    /// The page table used for context switch from the bootloader to the kernel.
    pub context_switch_page_table: OffsetPageTable<'static>,
    /// The physical frame where the level 4 page table of the context switch address space is stored.
    pub context_switch_page_table_frame: PhysFrame,
    /// Address just before the kernel's entrypoint.
    pub context_switch_addr: VirtAddr,
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
        let boot_info_layout = Layout::new::<BootInfo>();
        let regions = frame_allocator.len() + 4; // up to 4 regions might be split into used/unused
        let memory_regions_layout = Layout::array::<MemoryRegion>(regions).unwrap();
        let (combined, memory_regions_offset) =
            boot_info_layout.extend(memory_regions_layout).unwrap();

        let boot_info_addr = mapping_addr(
            config.mappings.boot_info,
            u64::from_usize(combined.size()),
            u64::from_usize(combined.align()),
            &mut mappings.used_entries,
        );
        assert!(
            boot_info_addr.is_aligned(u64::from_usize(combined.align())),
            "boot info addr is not properly aligned"
        );

        let memory_map_regions_addr = boot_info_addr + memory_regions_offset;
        let memory_map_regions_end = boot_info_addr + combined.size();

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
    let memory_regions = frame_allocator.construct_memory_map(
        memory_regions,
        mappings.kernel_slice_start,
        mappings.kernel_slice_len,
    );

    log::info!("Create bootinfo");

    // create boot info
    let boot_info = boot_info.write({
        let mut info = BootInfo::new(memory_regions.into());
        info.framebuffer = mappings
            .framebuffer
            .map(|addr| FrameBuffer::new(addr.as_u64(), system_info.framebuffer.expect("there shouldn't be a mapping for the framebuffer if there is not framebuffer").info))
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
        context_switch_trampoline: mappings.context_switch_trampoline,
        context_switch_page_table: mappings.context_switch_page_table_frame,
        context_switch_addr: mappings.context_switch_addr,
        page_table: kernel_level_4_frame,
        stack_top: mappings.stack_end.start_address(),
        boot_info,
    };

    log::info!(
        "Switching to kernel entry point at {:?}",
        mappings.entry_point
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
            "mov rsp, {}; sub rsp, 8; jmp {}",
            in(reg) addresses.stack_top.as_u64(),
            in(reg) addresses.context_switch_trampoline.as_u64(),
            in("rdx") addresses.context_switch_page_table.start_address().as_u64(),
            in("r13") addresses.context_switch_addr.as_u64(),
            in("rax") addresses.page_table.start_address().as_u64(),
            in("rdi") addresses.boot_info as *const _ as usize,
            options(noreturn),
        );
    }
}

/// Memory addresses required for the context switch.
struct Addresses {
    context_switch_trampoline: VirtAddr,
    context_switch_page_table: PhysFrame,
    context_switch_addr: VirtAddr,
    page_table: PhysFrame,
    stack_top: VirtAddr,
    boot_info: &'static mut BootInfo,
}

fn mapping_addr(
    mapping: Mapping,
    size: u64,
    alignment: u64,
    used_entries: &mut UsedLevel4Entries,
) -> VirtAddr {
    match mapping {
        Mapping::FixedAddress(addr) => VirtAddr::new(addr),
        Mapping::Dynamic => used_entries.get_free_address(size, alignment),
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
