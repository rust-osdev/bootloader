#![no_std]
#![no_main]

#[cfg(not(target_os = "none"))]
compile_error!("The bootloader crate must be compiled for the `x86_64-bootloader.json` target");

extern crate rlibc;

use bootloader::bootinfo::{BootInfo, FrameRange};
use core::arch::asm;
use core::{arch::global_asm, convert::TryInto, panic::PanicInfo};
use core::{mem, slice};
use fixedvec::alloc_stack;
use usize_conversions::usize_from;
use x86_64::instructions::tlb;
use x86_64::structures::paging::{
    frame::PhysFrameRange, page_table::PageTableEntry, Mapper, Page, PageTable, PageTableFlags,
    PageTableIndex, PhysFrame, RecursivePageTable, Size2MiB, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

// The bootloader_config.rs file contains some configuration constants set by the build script:
// PHYSICAL_MEMORY_OFFSET: The offset into the virtual address space where the physical memory
// is mapped if the `map_physical_memory` feature is activated.
//
// KERNEL_STACK_ADDRESS: The virtual address of the kernel stack.
//
// KERNEL_STACK_SIZE: The number of pages in the kernel stack.
include!(concat!(env!("OUT_DIR"), "/bootloader_config.rs"));

global_asm!(include_str!("stage_1.s"));
global_asm!(include_str!("stage_2.s"));
global_asm!(include_str!("e820.s"));
global_asm!(include_str!("stage_3.s"));

#[cfg(feature = "vga_320x200")]
global_asm!(include_str!("video_mode/vga_320x200.s"));
#[cfg(not(feature = "vga_320x200"))]
global_asm!(include_str!("video_mode/vga_text_80x25.s"));

unsafe fn context_switch(boot_info: VirtAddr, entry_point: VirtAddr, stack_pointer: VirtAddr) -> ! {
    asm!("mov rsp, {1}; call {0}; 2: jmp 2b",
         in(reg) entry_point.as_u64(), in(reg) stack_pointer.as_u64(), in("rdi") boot_info.as_u64());
    ::core::hint::unreachable_unchecked()
}

mod boot_info;
mod frame_allocator;
mod level4_entries;
mod page_table;
mod printer;
#[cfg(feature = "sse")]
mod sse;

pub struct IdentityMappedAddr(PhysAddr);

impl IdentityMappedAddr {
    fn phys(&self) -> PhysAddr {
        self.0
    }

    fn virt(&self) -> VirtAddr {
        VirtAddr::new(self.0.as_u64())
    }

    fn as_u64(&self) -> u64 {
        self.0.as_u64()
    }
}

// Symbols defined in `linker.ld`
extern "C" {
    static mmap_ent: usize;
    static _memory_map: usize;
    static _kernel_start_addr: usize;
    static _kernel_end_addr: usize;
    static _kernel_size: usize;
    static __page_table_start: usize;
    static __page_table_end: usize;
    static __bootloader_end: usize;
    static __bootloader_start: usize;
    static _p4: usize;
}

#[no_mangle]
pub unsafe extern "C" fn stage_4() -> ! {
    // Set stack segment
    asm!(
        "push rbx
          mov bx, 0x0
          mov ss, bx
          pop rbx"
    );

    let kernel_start = 0x400000;
    let kernel_size = &_kernel_size as *const _ as u64;
    let memory_map_addr = &_memory_map as *const _ as u64;
    let memory_map_entry_count = (mmap_ent & 0xff) as u64; // Extract lower 8 bits
    let page_table_start = &__page_table_start as *const _ as u64;
    let page_table_end = &__page_table_end as *const _ as u64;
    let bootloader_start = &__bootloader_start as *const _ as u64;
    let bootloader_end = &__bootloader_end as *const _ as u64;
    let p4_physical = &_p4 as *const _ as u64;

    bootloader_main(
        IdentityMappedAddr(PhysAddr::new(kernel_start)),
        kernel_size,
        VirtAddr::new(memory_map_addr),
        memory_map_entry_count,
        PhysAddr::new(page_table_start),
        PhysAddr::new(page_table_end),
        PhysAddr::new(bootloader_start),
        PhysAddr::new(bootloader_end),
        PhysAddr::new(p4_physical),
    )
}

fn bootloader_main(
    kernel_start: IdentityMappedAddr,
    kernel_size: u64,
    memory_map_addr: VirtAddr,
    memory_map_entry_count: u64,
    page_table_start: PhysAddr,
    page_table_end: PhysAddr,
    bootloader_start: PhysAddr,
    bootloader_end: PhysAddr,
    p4_physical: PhysAddr,
) -> ! {
    use bootloader::bootinfo::{MemoryRegion, MemoryRegionType};
    use fixedvec::FixedVec;
    use xmas_elf::program::{ProgramHeader, ProgramHeader64};

    printer::Printer.clear_screen();

    let mut memory_map = boot_info::create_from(memory_map_addr, memory_map_entry_count);

    let max_phys_addr = memory_map
        .iter()
        .map(|r| r.range.end_addr())
        .max()
        .expect("no physical memory regions found");

    // Extract required information from the ELF file.
    let mut preallocated_space = alloc_stack!([ProgramHeader64; 32]);
    let mut segments = FixedVec::new(&mut preallocated_space);
    let entry_point;
    {
        let kernel_start_ptr = usize_from(kernel_start.as_u64()) as *const u8;
        let kernel = unsafe { slice::from_raw_parts(kernel_start_ptr, usize_from(kernel_size)) };
        let elf_file = xmas_elf::ElfFile::new(kernel).unwrap();
        xmas_elf::header::sanity_check(&elf_file).unwrap();

        entry_point = elf_file.header.pt2.entry_point();

        for program_header in elf_file.program_iter() {
            match program_header {
                ProgramHeader::Ph64(header) => segments
                    .push(*header)
                    .expect("does not support more than 32 program segments"),
                ProgramHeader::Ph32(_) => panic!("does not support 32 bit elf files"),
            }
        }
    }

    // Mark used virtual addresses
    let mut level4_entries = level4_entries::UsedLevel4Entries::new(&segments);

    // Enable support for the no-execute bit in page tables.
    enable_nxe_bit();

    // Create a recursive page table entry
    let recursive_index =
        PageTableIndex::new(level4_entries.get_free_entries(1).try_into().unwrap());
    let mut entry = PageTableEntry::new();
    entry.set_addr(
        p4_physical,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    );

    // Write the recursive entry into the page table
    let page_table = unsafe { &mut *(p4_physical.as_u64() as *mut PageTable) };
    page_table[recursive_index] = entry;
    tlb::flush_all();

    let recursive_page_table_addr = Page::from_page_table_indices(
        recursive_index,
        recursive_index,
        recursive_index,
        recursive_index,
    )
    .start_address();
    let page_table = unsafe { &mut *(recursive_page_table_addr.as_mut_ptr()) };
    let mut rec_page_table =
        RecursivePageTable::new(page_table).expect("recursive page table creation failed");

    // Create a frame allocator, which marks allocated frames as used in the memory map.
    let mut frame_allocator = frame_allocator::FrameAllocator {
        memory_map: &mut memory_map,
    };

    // Mark already used memory areas in frame allocator.
    {
        let zero_frame: PhysFrame = PhysFrame::from_start_address(PhysAddr::new(0)).unwrap();
        frame_allocator.mark_allocated_region(MemoryRegion {
            range: frame_range(PhysFrame::range(zero_frame, zero_frame + 1)),
            region_type: MemoryRegionType::FrameZero,
        });
        let bootloader_start_frame = PhysFrame::containing_address(bootloader_start);
        let bootloader_end_frame = PhysFrame::containing_address(bootloader_end - 1u64);
        let bootloader_memory_area =
            PhysFrame::range(bootloader_start_frame, bootloader_end_frame + 1);
        frame_allocator.mark_allocated_region(MemoryRegion {
            range: frame_range(bootloader_memory_area),
            region_type: MemoryRegionType::Bootloader,
        });
        let kernel_start_frame = PhysFrame::containing_address(kernel_start.phys());
        let kernel_end_frame =
            PhysFrame::containing_address(kernel_start.phys() + kernel_size - 1u64);
        let kernel_memory_area = PhysFrame::range(kernel_start_frame, kernel_end_frame + 1);
        frame_allocator.mark_allocated_region(MemoryRegion {
            range: frame_range(kernel_memory_area),
            region_type: MemoryRegionType::Kernel,
        });
        let page_table_start_frame = PhysFrame::containing_address(page_table_start);
        let page_table_end_frame = PhysFrame::containing_address(page_table_end - 1u64);
        let page_table_memory_area =
            PhysFrame::range(page_table_start_frame, page_table_end_frame + 1);
        frame_allocator.mark_allocated_region(MemoryRegion {
            range: frame_range(page_table_memory_area),
            region_type: MemoryRegionType::PageTable,
        });
    }

    // Unmap the ELF file.
    let kernel_start_page: Page<Size2MiB> = Page::containing_address(kernel_start.virt());
    let kernel_end_page: Page<Size2MiB> =
        Page::containing_address(kernel_start.virt() + kernel_size - 1u64);
    for page in Page::range_inclusive(kernel_start_page, kernel_end_page) {
        rec_page_table.unmap(page).expect("dealloc error").1.flush();
    }

    // Map a page for the boot info structure
    let boot_info_page = {
        let page: Page = match BOOT_INFO_ADDRESS {
            Some(addr) => Page::containing_address(VirtAddr::new(addr)),
            None => Page::from_page_table_indices(
                level4_entries.get_free_entries(1),
                PageTableIndex::new(0),
                PageTableIndex::new(0),
                PageTableIndex::new(0),
            ),
        };
        let frame = frame_allocator
            .allocate_frame(MemoryRegionType::BootInfo)
            .expect("frame allocation failed");
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
        page
    };

    // If no kernel stack address is provided, map the kernel stack after the boot info page
    let kernel_stack_address = match KERNEL_STACK_ADDRESS {
        Some(addr) => Page::containing_address(VirtAddr::new(addr)),
        None => boot_info_page + 1,
    };

    // Map kernel segments.
    let kernel_memory_info = page_table::map_kernel(
        kernel_start.phys(),
        kernel_stack_address,
        KERNEL_STACK_SIZE,
        &segments,
        &mut rec_page_table,
        &mut frame_allocator,
    )
    .expect("kernel mapping failed");

    let physical_memory_offset = if cfg!(feature = "map_physical_memory") {
        let physical_memory_offset = PHYSICAL_MEMORY_OFFSET.unwrap_or_else(|| {
            const LEVEL_4_SIZE: u64 = 4096 * 512 * 512 * 512;
            let level_4_entries = (max_phys_addr + (LEVEL_4_SIZE - 1)) / LEVEL_4_SIZE;
            Page::from_page_table_indices_1gib(
                level4_entries.get_free_entries(level_4_entries),
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

    // Construct boot info structure.
    let mut boot_info = BootInfo::new(
        memory_map,
        kernel_memory_info.tls_segment,
        recursive_page_table_addr.as_u64(),
        physical_memory_offset,
    );
    boot_info.memory_map.sort();

    // Write boot info to boot info page.
    let boot_info_addr = boot_info_page.start_address();
    unsafe { boot_info_addr.as_mut_ptr::<BootInfo>().write(boot_info) };

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

    let entry_point = VirtAddr::new(entry_point);
    unsafe { context_switch(boot_info_addr, entry_point, kernel_memory_info.stack_end) };
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
pub fn panic(info: &PanicInfo) -> ! {
    use core::fmt::Write;
    write!(printer::Printer, "{}", info).unwrap();
    loop {}
}

#[no_mangle]
pub extern "C" fn _Unwind_Resume() {
    loop {}
}

fn phys_frame_range(range: FrameRange) -> PhysFrameRange {
    PhysFrameRange {
        start: PhysFrame::from_start_address(PhysAddr::new(range.start_addr())).unwrap(),
        end: PhysFrame::from_start_address(PhysAddr::new(range.end_addr())).unwrap(),
    }
}

fn frame_range(range: PhysFrameRange) -> FrameRange {
    FrameRange::new(
        range.start.start_address().as_u64(),
        range.end.start_address().as_u64(),
    )
}
