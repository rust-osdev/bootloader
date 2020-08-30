#![feature(lang_items)]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(asm)]
#![feature(slice_fill)]
#![no_std]
#![no_main]

#[cfg(not(target_os = "none"))]
compile_error!("The bootloader crate must be compiled for the `x86_64-bootloader.json` target");

extern crate rlibc;

use core::panic::PanicInfo;
use core::slice;
use usize_conversions::usize_from;
use x86_64::structures::paging::{FrameAllocator, OffsetPageTable};
use x86_64::structures::paging::{
    Mapper, PageTable, PageTableFlags, PhysFrame, Size2MiB, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

global_asm!(include_str!("../asm/stage_1.s"));
global_asm!(include_str!("../asm/stage_2.s"));
global_asm!(include_str!("../asm/e820.s"));
global_asm!(include_str!("../asm/stage_3.s"));

#[cfg(feature = "vga_320x200")]
global_asm!(include_str!("../asm/video_mode/vga_320x200.s"));
#[cfg(not(feature = "vga_320x200"))]
global_asm!(include_str!("../asm/video_mode/vga_text_80x25.s"));

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
    llvm_asm!("mov bx, 0x0
          mov ss, bx" ::: "bx" : "intel");

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

    let framebuffer_addr = PhysAddr::new(0xfd000000);
    let framebuffer_size = 1024 * 768;
    init_logger(framebuffer_addr, framebuffer_size);

    let page_tables = create_page_tables(&mut frame_allocator);

    let kernel = {
        let ptr = kernel_start.as_u64() as *const u8;
        unsafe { slice::from_raw_parts(ptr, usize_from(kernel_size)) }
    };

    bootloader::binary::load_and_switch_to_kernel(
        kernel,
        frame_allocator,
        page_tables,
        framebuffer_addr,
        usize_from(framebuffer_size),
    );

    /*
    // Enable support for the no-execute bit in page tables.
    enable_nxe_bit();


    let physical_memory_offset = if cfg!(feature = "map_physical_memory") {
        let physical_memory_offset = PHYSICAL_MEMORY_OFFSET.unwrap_or_else(|| {
            // If offset not manually provided, find a free p4 entry and map memory here.
            // One level 4 entry spans 2^48/512 bytes (over 500gib) so this should suffice.
            assert!(max_phys_addr < (1 << 48) / 512);
            Page::from_page_table_indices_1gib(
                level4_entries.get_free_entry(),
                PageTableIndex::new(0),
            )
            .start_address()
            .as_u64()
        });

        let virt_for_phys =
            |phys: PhysAddr| -> VirtAddr { VirtAddr::new(phys.as_u64() + physical_memory_offset) };

        let start_frame = PhysFrame::<Size2MiB>::containing_address(PhysAddr::new(0));
        let end_frame = PhysFrame::<Size2MiB>::containing_address(PhysAddr::new(max_phys_addr));

        for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
            let page = Page::containing_address(virt_for_phys(frame.start_address()));
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
            unsafe {
                page_table::map_page(
                    page,
                    frame,
                    flags,
                    &mut rec_page_table,
                    &mut frame_allocator,
                )
            }
            .expect("Mapping of bootinfo page failed")
            .flush();
        }

        physical_memory_offset
    } else {
        0 // Value is unused by BootInfo::new, so this doesn't matter
    };


    // Make sure that the kernel respects the write-protection bits, even when in ring 0.
    enable_write_protect_bit();

    if cfg!(not(feature = "recursive_page_table")) {
        // unmap recursive entry
        rec_page_table
            .unmap(Page::<Size4KiB>::containing_address(
                recursive_page_table_addr,
            ))
            .expect("error deallocating recursive entry")
            .1
            .flush();
        mem::drop(rec_page_table);
    }

    #[cfg(feature = "sse")]
    sse::enable_sse();

    */
}

fn init_logger(framebuffer_start: PhysAddr, framebuffer_size: u64) {
    let ptr = framebuffer_start.as_u64() as *mut u8;
    let slice = unsafe { slice::from_raw_parts_mut(ptr, usize_from(framebuffer_size)) };
    slice.fill(0x4);
    let info = bootloader::binary::logger::FrameBufferInfo {
        horizontal_resolution: 1024,
        vertical_resolution: 768,
        pixel_format: bootloader::binary::logger::PixelFormat::RGB,
        bytes_per_pixel: 3,
        stride: 1024,
    };

    bootloader::binary::init_logger(slice, info);
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

fn enable_nxe_bit() {
    use x86_64::registers::control::{Efer, EferFlags};
    unsafe { Efer::update(|efer| *efer |= EferFlags::NO_EXECUTE_ENABLE) }
}

fn enable_write_protect_bit() {
    use x86_64::registers::control::{Cr0, Cr0Flags};
    unsafe { Cr0::update(|cr0| *cr0 |= Cr0Flags::WRITE_PROTECT) };
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
