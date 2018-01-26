#![feature(lang_items)]
#![feature(global_asm)]
#![feature(iterator_step_by)]
#![feature(try_from)]
#![feature(step_trait)]
#![feature(asm)]
#![feature(nll)]
#![feature(pointer_methods)]

#![no_std]
#![no_main]

extern crate rlibc;
extern crate xmas_elf;
extern crate x86_64;
extern crate usize_conversions;
extern crate os_bootinfo;

pub use x86_64::PhysAddr;
use x86_64::VirtAddr;
use x86_64::structures::paging::{PAGE_SIZE, PageTable, PageTableFlags, PhysFrame, Page};
use core::slice;
use usize_conversions::{usize_from, FromUsize};
use os_bootinfo::BootInfo;

global_asm!(include_str!("boot.s"));
global_asm!(include_str!("second_stage.s"));
global_asm!(include_str!("memory_map.s"));
global_asm!(include_str!("context_switch.s"));

extern "C" {
    fn context_switch(p4_addr: PhysAddr, entry_point: VirtAddr, stack_pointer: VirtAddr,
        boot_info: VirtAddr) -> !;
}

mod boot_info;
mod page_table;

struct FrameAllocator {
    next: PhysFrame,
}

impl FrameAllocator {
    fn allocate_frame(&mut self) -> PhysFrame {
        let frame = self.next.clone();
        self.next += 1;
        frame
    }
}

#[no_mangle]
pub extern "C" fn load_elf(kernel_start: PhysAddr, kernel_size: u64,
    memory_map_addr: VirtAddr, memory_map_entry_count: u64) -> !
{
    let kernel_start_ptr = usize_from(kernel_start.as_u64()) as *const u8;
    let kernel = unsafe { slice::from_raw_parts(kernel_start_ptr, usize_from(kernel_size)) };
    let elf_file = xmas_elf::ElfFile::new(kernel).unwrap();
    xmas_elf::header::sanity_check(&elf_file).unwrap();

    // idea: embed memory map in frame allocator and mark allocated frames as used
    let mut boot_info = boot_info::create_from(memory_map_addr, memory_map_entry_count);
    let memory_map = &mut boot_info.memory_map;

    let kernel_end = kernel_start + kernel_size;
    let mut frame_allocator = FrameAllocator {
        next: PhysFrame::containing_address(kernel_end.align_up(PAGE_SIZE))
    };

    let p4_frame = frame_allocator.allocate_frame();
    let p4_addr = p4_frame.start_address();
    let p4 = unsafe { &mut *(usize_from(p4_addr.as_u64()) as *const PageTable as *mut PageTable) };

    let stack_end = page_table::map_kernel(kernel_start, &elf_file, p4, &mut frame_allocator);

    let boot_info_page = Page::containing_address(VirtAddr::new(0xf000000)); // TODO
    let boot_info_frame = frame_allocator.allocate_frame();
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    page_table::map_page(boot_info_page.clone(), boot_info_frame.clone(), flags, p4, &mut frame_allocator);

    // identity map VGA text buffer
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    let vga_frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    page_table::identity_map(vga_frame, flags, p4, &mut frame_allocator);

    // identity map context switch function to be able to switch P4 tables without page fault
    let context_switch_fn_addr = VirtAddr::new(u64::from_usize(
        context_switch as *const fn(PhysAddr, VirtAddr, VirtAddr, &'static BootInfo) -> ! as usize));
    let context_switch_fn_frame = PhysFrame::containing_address(
        PhysAddr::new(context_switch_fn_addr.as_u64()));
    let flags = PageTableFlags::PRESENT;
    page_table::identity_map(context_switch_fn_frame, flags, p4, &mut frame_allocator);

    let boot_info_addr = boot_info_page.start_address();
    let boot_info_ptr = usize_from(boot_info_frame.start_address().as_u64()) as *mut BootInfo;
    unsafe {boot_info_ptr.write(boot_info)};

    let entry_point = VirtAddr::new(elf_file.header.pt2.entry_point());
    unsafe { context_switch(p4_addr, entry_point, stack_end, boot_info_addr) };
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
