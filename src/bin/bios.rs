#![no_std]
#![no_main]

#[cfg(not(target_os = "none"))]
compile_error!("The bootloader crate must be compiled for the `x86_64-bootloader.json` target");

use bootloader::{
    binary::SystemInfo,
    boot_info::{FrameBufferInfo, PixelFormat},
};
use core::{
    arch::{asm, global_asm},
    panic::PanicInfo,
    slice,
};
use usize_conversions::usize_from;
use x86_64::structures::paging::{FrameAllocator, OffsetPageTable};
use x86_64::structures::paging::{
    Mapper, PageTable, PageTableFlags, PhysFrame, Size2MiB, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

global_asm!(include_str!("../asm/stage_1.s"));
global_asm!(include_str!("../asm/stage_2.s"));
global_asm!(include_str!(concat!(env!("OUT_DIR"), "/vesa_config.s")));
global_asm!(include_str!("../asm/vesa.s"));
global_asm!(include_str!("../asm/e820.s"));
global_asm!(include_str!("../asm/stage_3.s"));

// values defined in `vesa.s`
extern "C" {
    static VBEModeInfo_physbaseptr: u32;
    static VBEModeInfo_bytesperscanline: u16;
    static VBEModeInfo_xresolution: u16;
    static VBEModeInfo_yresolution: u16;
    static VBEModeInfo_bitsperpixel: u8;
    static VBEModeInfo_redfieldposition: u8;
    static VBEModeInfo_greenfieldposition: u8;
    static VBEModeInfo_bluefieldposition: u8;
}

// Symbols defined in `linker.ld`
extern "C" {
    static mmap_ent: usize;
    static _memory_map: usize;
    static _kernel_start_addr: usize;
    static _kernel_end_addr: usize;
    static _kernel_size: usize;
}

#[no_mangle]
pub unsafe extern "C" fn stage_4() -> ! {
    // Set stack segment
    asm!(
        "mov ax, 0x0; mov ss, ax",
        out("ax") _,
    );

    let kernel_start = 0x400000;
    let kernel_size = &_kernel_size as *const _ as u64;
    let memory_map_addr = &_memory_map as *const _ as u64;
    let memory_map_entry_count = (mmap_ent & 0xff) as u64; // Extract lower 8 bits

    bootloader_main(
        PhysAddr::new(kernel_start),
        kernel_size,
        VirtAddr::new(memory_map_addr),
        memory_map_entry_count,
    )
}

fn bootloader_main(
    kernel_start: PhysAddr,
    kernel_size: u64,
    memory_map_addr: VirtAddr,
    memory_map_entry_count: u64,
) -> ! {
    use bootloader::binary::{
        bios::memory_descriptor::E820MemoryRegion, legacy_memory_region::LegacyFrameAllocator,
    };

    let e820_memory_map = {
        let ptr = usize_from(memory_map_addr.as_u64()) as *const E820MemoryRegion;
        unsafe { slice::from_raw_parts(ptr, usize_from(memory_map_entry_count)) }
    };
    let max_phys_addr = e820_memory_map
        .iter()
        .map(|r| r.start_addr + r.len)
        .max()
        .expect("no physical memory regions found");

    let mut frame_allocator = {
        let kernel_end = PhysFrame::containing_address(kernel_start + kernel_size - 1u64);
        let next_free = kernel_end + 1;
        LegacyFrameAllocator::new_starting_at(next_free, e820_memory_map.iter().copied())
    };

    // We identity-map all memory, so the offset between physical and virtual addresses is 0
    let phys_offset = VirtAddr::new(0);

    let mut bootloader_page_table = {
        let frame = x86_64::registers::control::Cr3::read().0;
        let table: *mut PageTable = (phys_offset + frame.start_address().as_u64()).as_mut_ptr();
        unsafe { OffsetPageTable::new(&mut *table, phys_offset) }
    };
    // identity-map remaining physical memory (first gigabyte is already identity-mapped)
    {
        let start_frame: PhysFrame<Size2MiB> =
            PhysFrame::containing_address(PhysAddr::new(4096 * 512 * 512));
        let end_frame = PhysFrame::containing_address(PhysAddr::new(max_phys_addr - 1));
        for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
            unsafe {
                bootloader_page_table
                    .identity_map(
                        frame,
                        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                        &mut frame_allocator,
                    )
                    .unwrap()
                    .flush()
            };
        }
    }

    let framebuffer_addr = PhysAddr::new(unsafe { VBEModeInfo_physbaseptr }.into());
    let mut error = None;
    let framebuffer_info = unsafe {
        let framebuffer_size =
            usize::from(VBEModeInfo_yresolution) * usize::from(VBEModeInfo_bytesperscanline);
        let bytes_per_pixel = VBEModeInfo_bitsperpixel / 8;
        init_logger(
            framebuffer_addr,
            framebuffer_size.into(),
            VBEModeInfo_xresolution.into(),
            VBEModeInfo_yresolution.into(),
            bytes_per_pixel.into(),
            (VBEModeInfo_bytesperscanline / u16::from(bytes_per_pixel)).into(),
            match (
                VBEModeInfo_redfieldposition,
                VBEModeInfo_greenfieldposition,
                VBEModeInfo_bluefieldposition,
            ) {
                (0, 8, 16) => PixelFormat::RGB,
                (16, 8, 0) => PixelFormat::BGR,
                (r, g, b) => {
                    error = Some(("invalid rgb field positions", r, g, b));
                    PixelFormat::RGB // default to RBG so that we can print something
                }
            },
        )
    };

    log::info!("BIOS boot");

    if let Some((msg, r, g, b)) = error {
        panic!("{}: r: {}, g: {}, b: {}", msg, r, g, b);
    }

    let page_tables = create_page_tables(&mut frame_allocator);

    let kernel = {
        let ptr = kernel_start.as_u64() as *const u8;
        unsafe { slice::from_raw_parts(ptr, usize_from(kernel_size)) }
    };

    let system_info = SystemInfo {
        framebuffer_addr,
        framebuffer_info,
        rsdp_addr: detect_rsdp(),
    };

    bootloader::binary::load_and_switch_to_kernel(
        kernel,
        frame_allocator,
        page_tables,
        system_info,
    );
}

fn init_logger(
    framebuffer_start: PhysAddr,
    framebuffer_size: usize,
    horizontal_resolution: usize,
    vertical_resolution: usize,
    bytes_per_pixel: usize,
    stride: usize,
    pixel_format: PixelFormat,
) -> FrameBufferInfo {
    let ptr = framebuffer_start.as_u64() as *mut u8;
    let slice = unsafe { slice::from_raw_parts_mut(ptr, framebuffer_size) };

    let info = bootloader::boot_info::FrameBufferInfo {
        byte_len: framebuffer_size,
        horizontal_resolution,
        vertical_resolution,
        bytes_per_pixel,
        stride,
        pixel_format,
    };

    bootloader::binary::init_logger(slice, info);

    info
}

/// Creates page table abstraction types for both the bootloader and kernel page tables.
fn create_page_tables(
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> bootloader::binary::PageTables {
    // We identity-mapped all memory, so the offset between physical and virtual addresses is 0
    let phys_offset = VirtAddr::new(0);

    // copy the currently active level 4 page table, because it might be read-only
    let bootloader_page_table = {
        let frame = x86_64::registers::control::Cr3::read().0;
        let table: *mut PageTable = (phys_offset + frame.start_address().as_u64()).as_mut_ptr();
        unsafe { OffsetPageTable::new(&mut *table, phys_offset) }
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

fn detect_rsdp() -> Option<PhysAddr> {
    use core::ptr::NonNull;
    use rsdp::{
        handler::{AcpiHandler, PhysicalMapping},
        Rsdp,
    };

    #[derive(Clone)]
    struct IdentityMapped;
    impl AcpiHandler for IdentityMapped {
        unsafe fn map_physical_region<T>(
            &self,
            physical_address: usize,
            size: usize,
        ) -> PhysicalMapping<Self, T> {
            PhysicalMapping {
                physical_start: physical_address,
                virtual_start: NonNull::new(physical_address as *mut _).unwrap(),
                region_length: size,
                mapped_length: size,
                handler: Self,
            }
        }

        fn unmap_physical_region<T>(&self, _region: &PhysicalMapping<Self, T>) {}
    }

    unsafe {
        Rsdp::search_for_on_bios(IdentityMapped)
            .ok()
            .map(|mapping| PhysAddr::new(mapping.physical_start as u64))
    }
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
