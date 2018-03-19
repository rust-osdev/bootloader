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

extern crate rlibc;
extern crate xmas_elf;
extern crate x86_64;
extern crate usize_conversions;
extern crate os_bootinfo;
extern crate spin;
#[macro_use]
extern crate fixedvec;

pub use x86_64::PhysAddr;
use x86_64::VirtAddr;
use x86_64::ux::u9;
use x86_64::structures::paging::{PAGE_SIZE, PageTableFlags, Page};
use x86_64::structures::paging::RecursivePageTable;
use x86_64::instructions::tlb;
use core::slice;
use usize_conversions::usize_from;
use os_bootinfo::BootInfo;

global_asm!(include_str!("boot.s"));
global_asm!(include_str!("second_stage.s"));
global_asm!(include_str!("memory_map.s"));
global_asm!(include_str!("context_switch.s"));

extern "C" {
    fn context_switch(boot_info: VirtAddr, entry_point: VirtAddr, stack_pointer: VirtAddr) -> !;
}

mod boot_info;
mod page_table;
mod frame_allocator;
mod printer;

#[no_mangle]
pub extern "C" fn load_elf(kernel_start: PhysAddr, kernel_size: u64,
        memory_map_addr: VirtAddr, memory_map_entry_count: u64,
        page_table_start: PhysAddr, page_table_end: PhysAddr,
        bootloader_start: PhysAddr, bootloader_end: PhysAddr,
    ) -> !
{
    use fixedvec::FixedVec;
    use xmas_elf::program::{ProgramHeader, ProgramHeader64};
    use os_bootinfo::{MemoryRegion, MemoryRegionType};

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
                ProgramHeader::Ph64(header) => {
                    segments.push(*header).expect("does not support more than 32 program segments")
                },
                ProgramHeader::Ph32(_) => panic!("does not support 32 bit elf files"),
            }
        }
    }

    // Enable support for the no-execute bit in page tables.
    enable_nxe_bit();

    // Create a RecursivePageTable
    let recursive_index = u9::new(511);
    let recursive_page_table_addr = Page::from_page_table_indices(recursive_index,
        recursive_index, recursive_index, recursive_index);
    let page_table = unsafe { &mut *(recursive_page_table_addr.start_address().as_mut_ptr()) };
    let mut rec_page_table = RecursivePageTable::new(page_table)
        .expect("recursive page table creation failed");

    // Create a frame allocator, which marks allocated frames as used in the memory map.
    let mut frame_allocator = frame_allocator::FrameAllocator{ memory_map:&mut memory_map };

    // Mark already used memory areas in frame allocator.
    {
        frame_allocator.add_region(MemoryRegion {
            start_addr: kernel_start, len: kernel_size, region_type: MemoryRegionType::Kernel,
        });
        frame_allocator.add_region(MemoryRegion {
            start_addr: page_table_start, len: page_table_end - page_table_start,
            region_type: MemoryRegionType::PageTable,
        });
        frame_allocator.add_region(MemoryRegion {
            start_addr: bootloader_start, len: bootloader_end - bootloader_start,
            region_type: MemoryRegionType::Bootloader,
        });
        frame_allocator.add_region(MemoryRegion {
            start_addr: PhysAddr::new(0), len: u64::from(PAGE_SIZE),
            region_type: MemoryRegionType::FrameZero,
        });
    }

    // Unmap the ELF file.
    const _2MIB: usize = 2*1024*1024;
    for addr in (kernel_start..kernel_start+kernel_size).step_by(_2MIB) {
        let page = Page::containing_address(VirtAddr::new(addr.as_u64()));
        rec_page_table.unmap(page, &mut |frame| {
            frame_allocator.deallocate_frame(frame);
        }).expect("dealloc error");
    }
    // Flush the translation lookaside buffer since we changed the active mapping.
    tlb::flush_all();

    // Map kernel segments.
    let stack_end = page_table::map_kernel(kernel_start, &segments, &mut rec_page_table,
        &mut frame_allocator).expect("kernel mapping failed");

    // Map a page for the boot info structure
    let boot_info_page = Page::containing_address(VirtAddr::new(0xb0071f0000));
    let boot_info_frame = frame_allocator.allocate_frame(MemoryRegionType::Bootloader)
        .expect("frame allocation failed");
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    page_table::map_page(boot_info_page.clone(), boot_info_frame.clone(), flags,
        &mut rec_page_table, &mut frame_allocator).expect("Mapping of bootinfo page failed");

    // Construct boot info structure.
    let mut boot_info = BootInfo::new(page_table, memory_map);
    boot_info.sort_memory_map();

    // Write boot info to boot info page.
    let boot_info_addr = boot_info_page.start_address();
    let boot_info_ptr = usize_from(boot_info_frame.start_address().as_u64()) as *mut BootInfo;
    unsafe {boot_info_ptr.write(boot_info)};

    // Make sure that the kernel respects the write-protection bits, even when in ring 0.
    enable_write_protect_bit();

    let entry_point = VirtAddr::new(entry_point);
    printer::PRINTER.lock().clear_screen();

    unsafe { context_switch(boot_info_addr, entry_point, stack_end) };
}

fn enable_nxe_bit() {
    use x86_64::registers::control::{Efer, EferFlags};
    unsafe { Efer::update(|efer| *efer |= EferFlags::NO_EXECUTE_ENABLE)}
}

fn enable_write_protect_bit() {
    use x86_64::registers::control::{Cr0, Cr0Flags};
    unsafe { Cr0::update(|cr0| *cr0 |= Cr0Flags::WRITE_PROTECT) };
}

#[lang = "panic_fmt"]
#[no_mangle]
pub extern fn rust_begin_panic(msg: core::fmt::Arguments,
                               _file: &'static str,
                               _line: u32,
                               _column: u32) -> ! {
    use core::fmt::Write;
    write!(printer::PRINTER.lock(), "PANIC: {}", msg).unwrap();

    loop {}
}

#[lang = "eh_personality"]
#[no_mangle]
pub extern fn eh_personality() { loop {} }

#[no_mangle]
pub extern fn _Unwind_Resume() { loop {} }
