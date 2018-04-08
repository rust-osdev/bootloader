#![feature(lang_items)]
#![feature(global_asm)]
#![feature(iterator_step_by)]
#![feature(try_from)]
#![feature(step_trait)]
#![feature(asm)]
#![feature(nll)]
#![feature(pointer_methods)]
#![feature(const_fn)]
#![feature(nll)]
#![no_std]
#![no_main]

extern crate os_bootinfo;
extern crate usize_conversions;
extern crate x86_64;
extern crate xmas_elf;
#[macro_use]
extern crate fixedvec;

use core::slice;
use os_bootinfo::BootInfo;
use usize_conversions::usize_from;
pub use x86_64::PhysAddr;
use x86_64::VirtAddr;
use x86_64::instructions::tlb;
use x86_64::structures::paging::RecursivePageTable;
use x86_64::structures::paging::{Page, PageTableFlags, PAGE_SIZE};
use x86_64::ux::u9;

global_asm!(include_str!("boot.s"));
global_asm!(include_str!("second_stage.s"));
global_asm!(include_str!("memory_map.s"));
global_asm!(include_str!("context_switch.s"));

extern "C" {
    fn context_switch(boot_info: VirtAddr, entry_point: VirtAddr, stack_pointer: VirtAddr) -> !;
}

mod boot_info;
mod frame_allocator;
mod page_table;
mod printer;

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

#[no_mangle]
pub extern "C" fn load_elf(
    kernel_start: IdentityMappedAddr,
    kernel_size: u64,
    memory_map_addr: VirtAddr,
    memory_map_entry_count: u64,
    page_table_start: PhysAddr,
    page_table_end: PhysAddr,
    bootloader_start: PhysAddr,
    bootloader_end: PhysAddr,
) -> ! {
    use fixedvec::FixedVec;
    use os_bootinfo::{MemoryRegion, MemoryRegionType};
    use xmas_elf::program::{ProgramHeader, ProgramHeader64};

    printer::Printer.clear_screen();

    let mut memory_map = boot_info::create_from(memory_map_addr, memory_map_entry_count);

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

    // Enable support for the no-execute bit in page tables.
    enable_nxe_bit();

    // Create a RecursivePageTable
    let recursive_index = u9::new(511);
    let recursive_page_table_addr = Page::from_page_table_indices(
        recursive_index,
        recursive_index,
        recursive_index,
        recursive_index,
    );
    let page_table = unsafe { &mut *(recursive_page_table_addr.start_address().as_mut_ptr()) };
    let mut rec_page_table =
        RecursivePageTable::new(page_table).expect("recursive page table creation failed");

    // Create a frame allocator, which marks allocated frames as used in the memory map.
    let mut frame_allocator = frame_allocator::FrameAllocator {
        memory_map: &mut memory_map,
    };

    // Mark already used memory areas in frame allocator.
    {
        frame_allocator.add_region(MemoryRegion {
            start_addr: kernel_start.phys(),
            len: kernel_size,
            region_type: MemoryRegionType::Kernel,
        });
        frame_allocator.add_region(MemoryRegion {
            start_addr: page_table_start,
            len: page_table_end - page_table_start,
            region_type: MemoryRegionType::PageTable,
        });
        frame_allocator.add_region(MemoryRegion {
            start_addr: bootloader_start,
            len: bootloader_end - bootloader_start,
            region_type: MemoryRegionType::Bootloader,
        });
        frame_allocator.add_region(MemoryRegion {
            start_addr: PhysAddr::new(0),
            len: u64::from(PAGE_SIZE),
            region_type: MemoryRegionType::FrameZero,
        });
    }

    // Unmap the ELF file.
    let kernel_start_page = Page::containing_address(kernel_start.virt());
    let kernel_end_page = Page::containing_address(kernel_start.virt() + kernel_size - 1u64);
    for page in Page::range_inclusive(kernel_start_page, kernel_end_page).step_by(512) {
        rec_page_table
            .unmap(page, &mut |frame| {
                frame_allocator.deallocate_frame(frame);
            })
            .expect("dealloc error");
    }
    // Flush the translation lookaside buffer since we changed the active mapping.
    tlb::flush_all();

    // Map kernel segments.
    let stack_end = page_table::map_kernel(
        kernel_start.phys(),
        &segments,
        &mut rec_page_table,
        &mut frame_allocator,
    ).expect("kernel mapping failed");

    // Map a page for the boot info structure
    let boot_info_page = {
        let page = Page::containing_address(VirtAddr::new(0xb0071f0000));
        let frame = frame_allocator
            .allocate_frame(MemoryRegionType::Bootloader)
            .expect("frame allocation failed");
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        page_table::map_page(
            page,
            frame,
            flags,
            &mut rec_page_table,
            &mut frame_allocator,
        ).expect("Mapping of bootinfo page failed");
        page
    };

    // Construct boot info structure.
    let mut boot_info = BootInfo::new(page_table, memory_map);
    boot_info.memory_map.sort();

    // Write boot info to boot info page.
    let boot_info_addr = boot_info_page.start_address();
    unsafe { boot_info_addr.as_mut_ptr::<BootInfo>().write(boot_info) };

    // Make sure that the kernel respects the write-protection bits, even when in ring 0.
    enable_write_protect_bit();

    let entry_point = VirtAddr::new(entry_point);

    unsafe { context_switch(boot_info_addr, entry_point, stack_end) };
}

fn enable_nxe_bit() {
    use x86_64::registers::control::{Efer, EferFlags};
    unsafe { Efer::update(|efer| *efer |= EferFlags::NO_EXECUTE_ENABLE) }
}

fn enable_write_protect_bit() {
    use x86_64::registers::control::{Cr0, Cr0Flags};
    unsafe { Cr0::update(|cr0| *cr0 |= Cr0Flags::WRITE_PROTECT) };
}

#[lang = "panic_fmt"]
#[no_mangle]
pub extern "C" fn rust_begin_panic(
    msg: core::fmt::Arguments,
    _file: &'static str,
    _line: u32,
    _column: u32,
) -> ! {
    use core::fmt::Write;
    write!(printer::Printer, "PANIC: {}", msg).unwrap();

    loop {}
}

#[lang = "eh_personality"]
#[no_mangle]
pub extern "C" fn eh_personality() {
    loop {}
}

#[no_mangle]
pub extern "C" fn _Unwind_Resume() {
    loop {}
}
