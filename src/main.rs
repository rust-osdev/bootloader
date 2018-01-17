#![feature(lang_items)]
#![feature(global_asm)]
#![feature(iterator_step_by)]
#![feature(try_from)]
#![feature(step_trait)]
#![feature(asm)]
#![no_std]
#![no_main]

extern crate rlibc;
extern crate xmas_elf;
extern crate x86_64;
extern crate usize_conversions;

use core::slice;
use usize_conversions::usize_from;
use xmas_elf::program::{self, ProgramHeader};

global_asm!(include_str!("boot.s"));
global_asm!(include_str!("second_stage.s"));
global_asm!(include_str!("kernel.s"));
global_asm!(include_str!("context_switch.s"));

extern "C" {
    fn context_switch(p4_addr: PhysAddr, entry_point: VirtAddr, stack_pointer: VirtAddr) -> !;
}

pub use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{PAGE_SIZE, PageTable, PageTableFlags, PageTableEntry, PhysFrame};

#[no_mangle]
pub extern "C" fn load_elf(kernel_start: PhysAddr, kernel_size: u64) -> ! {
    let kernel_end = kernel_start + kernel_size;
    let p4_addr = kernel_end.align_up(PAGE_SIZE);
    let p4 = unsafe { &mut *(usize_from(p4_addr.as_u64()) as *const PageTable as *mut PageTable) };
    let mut page_table_end = p4_addr + u64::from(PAGE_SIZE);

    let kernel_start_ptr = usize_from(kernel_start.as_u64()) as *const u8;
    let kernel = unsafe { slice::from_raw_parts(kernel_start_ptr, usize_from(kernel_size)) };

    let elf_file = xmas_elf::ElfFile::new(kernel).unwrap();
    xmas_elf::header::sanity_check(&elf_file).unwrap();

    for program_header in elf_file.program_iter() {
        map_segment(kernel_start, program_header, p4, &mut page_table_end);
    }

    // create a stack
    // TODO create a stack range dynamically (based on where the kernel is loaded)
    let stack_start = VirtAddr::new(0x57AC_0000_0000);
    let stack_size = 1 * 1024 * 1024; // 1 MiB
    let stack_end = stack_start + stack_size;

    let phys_stack_start = page_table_end;
    page_table_end += stack_size;

    let page_size = usize_from(PAGE_SIZE);
    let virt_page_iter = (stack_start..(stack_start + stack_size)).step_by(page_size);
    let phys_page_iter = (phys_stack_start..(phys_stack_start + stack_size)).step_by(page_size);
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    for (virt_page_addr, phys_page_addr) in virt_page_iter.zip(phys_page_iter) {
        map_page(virt_page_addr, phys_page_addr, flags, p4, &mut page_table_end);
    }

    // identity map context switch function to be able to switch P4 tables without page fault
    let context_switch_fn_addr = context_switch as *const u8 as u64;
    let context_switch_fn_virt = VirtAddr::new(context_switch_fn_addr);
    let context_switch_fn_phys = PhysAddr::new(context_switch_fn_addr);
    let flags = PageTableFlags::PRESENT;
    map_page(context_switch_fn_virt, context_switch_fn_phys, flags, p4, &mut page_table_end);

    // identity map VGA text buffer
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    map_page(VirtAddr::new(0xb8000), PhysAddr::new(0xb8000), flags, p4, &mut page_table_end);

    let entry_point = VirtAddr::new(elf_file.header.pt2.entry_point());
    unsafe { context_switch(p4_addr, entry_point, stack_end) };
}

fn map_segment(kernel_start: PhysAddr, program_header: ProgramHeader, p4: &mut PageTable,
    page_table_end: &mut PhysAddr)
{
    let typ = program_header.get_type().unwrap();
    match typ {
        program::Type::Load => {
            let offset = program_header.offset();
            let phys_start_addr = kernel_start + offset;
            let size = program_header.mem_size();
            let virt_start_addr = {
                let v = program_header.virtual_addr();
                VirtAddr::new(v)
            };
            let flags = program_header.flags();
            let mut page_table_flags = PageTableFlags::PRESENT;
            if !flags.is_execute() { page_table_flags |= PageTableFlags::NO_EXECUTE };
            if flags.is_write() { page_table_flags |= PageTableFlags::WRITABLE };

            let page_size = usize_from(PAGE_SIZE);
            let virt_page_iter = (virt_start_addr..(virt_start_addr + size)).step_by(page_size);
            let phys_page_iter = (phys_start_addr..(phys_start_addr + size)).step_by(page_size);

            for (virt_page_addr, phys_page_addr) in virt_page_iter.zip(phys_page_iter) {
                map_page(virt_page_addr, phys_page_addr, page_table_flags, p4, page_table_end);
            }
        },
        _ => {},
    }
}

fn map_page(virt_page_addr: VirtAddr, phys_page_addr: PhysAddr, flags: PageTableFlags,
    p4: &mut PageTable, page_table_end: &mut PhysAddr)
{
    fn as_page_table_ptr(addr: PhysAddr) -> *mut PageTable {
        usize_from(addr.as_u64()) as *const PageTable as *mut PageTable
    }

    fn create_and_link_page_table(page_table_end: &mut PhysAddr,
        parent_table_entry: &mut PageTableEntry) -> &'static mut PageTable
    {
        let table_frame = PhysFrame::containing_address(*page_table_end);
        *page_table_end += u64::from(PAGE_SIZE);
        let page_table = unsafe { &mut *as_page_table_ptr(table_frame.start_address()) };
        page_table.zero();
        parent_table_entry.set(table_frame, PageTableFlags::PRESENT);
        page_table
    }

    fn get_or_create_next_page_table(page_table_end: &mut PhysAddr,
        page_table_entry: &mut PageTableEntry) -> &'static mut PageTable
    {
        match page_table_entry.points_to() {
            Some(addr) => unsafe { &mut *as_page_table_ptr(addr) },
            None => create_and_link_page_table(page_table_end, page_table_entry)
        }
    }

    let p4_entry = &mut p4[virt_page_addr.p4_index()];
    let p3 = get_or_create_next_page_table(page_table_end, p4_entry);

    let p3_entry = &mut p3[virt_page_addr.p3_index()];
    let p2 = get_or_create_next_page_table(page_table_end, p3_entry);

    let p2_entry = &mut p2[virt_page_addr.p2_index()];
    let p1 = get_or_create_next_page_table(page_table_end, p2_entry);

    let p1_entry = &mut p1[virt_page_addr.p1_index()];
    assert!(p1_entry.is_unused(), "page for {:?} already in use", virt_page_addr);
    p1_entry.set(PhysFrame::containing_address(phys_page_addr), flags);
}

#[lang = "panic_fmt"]
#[no_mangle]
pub extern fn rust_begin_panic(_msg: core::fmt::Arguments,
                               _file: &'static str,
                               _line: u32,
                               _column: u32) -> ! {
    const VGA_BUFFER: *mut u8 = 0xb8000 as *mut _;

    unsafe {
        let vga_buffer = slice::from_raw_parts_mut(VGA_BUFFER, 25 * 80 *2);
        vga_buffer[0] = b'E'; vga_buffer[1] = 0x4f;
        vga_buffer[2] = b'R'; vga_buffer[3] = 0x4f;
        vga_buffer[4] = b'R'; vga_buffer[5] = 0x4f;
    }

    loop {}
}

#[lang = "eh_personality"]
#[no_mangle]
pub extern fn eh_personality() { loop {} }

#[no_mangle]
pub extern fn _Unwind_Resume() { loop {} }
