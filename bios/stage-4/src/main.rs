#![no_std]
#![no_main]

use crate::memory_descriptor::MemoryRegion;
use bootloader_api::info::{FrameBufferInfo, PixelFormat};
use bootloader_x86_64_bios_common::{BiosFramebufferInfo, BiosInfo, E820MemoryRegion};
use bootloader_x86_64_common::{
    legacy_memory_region::LegacyFrameAllocator, load_and_switch_to_kernel, Kernel, PageTables,
    SystemInfo,
};
use core::slice;
use usize_conversions::usize_from;
use x86_64::structures::paging::{FrameAllocator, OffsetPageTable};
use x86_64::structures::paging::{
    Mapper, PageTable, PageTableFlags, PhysFrame, Size2MiB, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

mod memory_descriptor;

#[no_mangle]
#[link_section = ".start"]
pub extern "C" fn _start(info: &mut BiosInfo) -> ! {
    let framebuffer_info = init_logger(info.framebuffer);
    log::info!("4th Stage");
    log::info!("{info:x?}");

    let memory_map: &mut [E820MemoryRegion] = unsafe {
        core::slice::from_raw_parts_mut(
            info.memory_map_addr as *mut _,
            info.memory_map_len.try_into().unwrap(),
        )
    };

    memory_map.sort_unstable_by_key(|e| e.start_addr);

    let max_phys_addr = memory_map
        .iter()
        .map(|r| {
            log::info!("start: {:#x}, len: {:#x}", r.start_addr, r.len);
            r.start_addr + r.len
        })
        .max()
        .expect("no physical memory regions found");

    let kernel_start = {
        assert!(info.kernel.start != 0, "kernel start address must be set");
        PhysAddr::new(info.kernel.start)
    };
    let kernel_size = info.kernel.len;
    let mut frame_allocator = {
        let kernel_end = PhysFrame::containing_address(kernel_start + kernel_size - 1u64);
        let next_free = kernel_end + 1;
        LegacyFrameAllocator::new_starting_at(
            next_free,
            memory_map.iter().copied().map(MemoryRegion),
        )
    };

    // We identity-mapped all memory, so the offset between physical and virtual addresses is 0
    let phys_offset = VirtAddr::new(0);

    let mut bootloader_page_table = {
        let frame = x86_64::registers::control::Cr3::read().0;
        let table: *mut PageTable = (phys_offset + frame.start_address().as_u64()).as_mut_ptr();
        unsafe { OffsetPageTable::new(&mut *table, phys_offset) }
    };
    // identity-map remaining physical memory (first 10 gigabytes are already identity-mapped)
    {
        let start_frame: PhysFrame<Size2MiB> =
            PhysFrame::containing_address(PhysAddr::new(4096 * 512 * 512 * 10));
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

    log::info!("BIOS boot");

    let page_tables = create_page_tables(&mut frame_allocator);

    let kernel_slice = {
        let ptr = kernel_start.as_u64() as *const u8;
        unsafe { slice::from_raw_parts(ptr, usize_from(kernel_size)) }
    };
    let kernel = Kernel::parse(kernel_slice);

    let system_info = SystemInfo {
        framebuffer_addr: PhysAddr::new(info.framebuffer.region.start),
        framebuffer_info,
        rsdp_addr: detect_rsdp(),
    };

    load_and_switch_to_kernel(kernel, frame_allocator, page_tables, system_info);
}

fn init_logger(info: BiosFramebufferInfo) -> FrameBufferInfo {
    let framebuffer_info = FrameBufferInfo {
        byte_len: info.region.len.try_into().unwrap(),
        width: info.width.into(),
        height: info.height.into(),
        pixel_format: match info.pixel_format {
            bootloader_x86_64_bios_common::PixelFormat::Rgb => PixelFormat::Rgb,
            bootloader_x86_64_bios_common::PixelFormat::Bgr => PixelFormat::Bgr,
            bootloader_x86_64_bios_common::PixelFormat::Unknown {
                red_position,
                green_position,
                blue_position,
            } => PixelFormat::Unknown {
                red_position,
                green_position,
                blue_position,
            },
        },
        bytes_per_pixel: info.bytes_per_pixel.into(),
        stride: info.stride.into(),
    };

    let framebuffer = unsafe {
        core::slice::from_raw_parts_mut(
            info.region.start as *mut u8,
            info.region.len.try_into().unwrap(),
        )
    };

    bootloader_x86_64_common::init_logger(framebuffer, framebuffer_info);

    framebuffer_info
}

/// Creates page table abstraction types for both the bootloader and kernel page tables.
fn create_page_tables(frame_allocator: &mut impl FrameAllocator<Size4KiB>) -> PageTables {
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
        log::info!("New page table at: {frame:#?}");
        // get the corresponding virtual address
        let addr = phys_offset + frame.start_address().as_u64();
        // initialize a new page table
        let ptr: *mut PageTable = addr.as_mut_ptr();
        unsafe { ptr.write(PageTable::new()) };
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
            PhysicalMapping::new(
                physical_address,
                NonNull::new(physical_address as *mut _).unwrap(),
                size,
                size,
                Self,
            )
        }

        fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}
    }

    unsafe {
        Rsdp::search_for_on_bios(IdentityMapped)
            .ok()
            .map(|mapping| PhysAddr::new(mapping.physical_start() as u64))
    }
}

#[panic_handler]
#[cfg(not(test))]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        bootloader_x86_64_common::logger::LOGGER
            .get()
            .map(|l| l.force_unlock())
    };
    log::error!("{info}");
    loop {
        unsafe { core::arch::asm!("cli; hlt") };
    }
}
