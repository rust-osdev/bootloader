use frame_allocator::FrameAllocator;
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{PAGE_SIZE, PageTable, PageTableFlags, PageTableEntry, Page, PhysFrame};
use usize_conversions::usize_from;
use xmas_elf;
use xmas_elf::program::{self, ProgramHeader};

pub(crate) fn map_kernel(kernel_start: PhysAddr, elf_file: &xmas_elf::ElfFile, p4: &mut PageTable,
    frame_allocator: &mut FrameAllocator) -> VirtAddr
{
    for program_header in elf_file.program_iter() {
        map_segment(kernel_start, program_header, p4, frame_allocator);
    }

    // create a stack
    // TODO create a stack range dynamically (based on where the kernel is loaded)
    let stack_start = VirtAddr::new(0x57AC_0000_0000);
    let stack_size = 1 * 1024 * 1024;
    let stack_end = stack_start + stack_size;

    let page_size = usize_from(PAGE_SIZE);
    let virt_page_iter = (stack_start..(stack_start + stack_size)).step_by(page_size);
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    for virt_page_addr in virt_page_iter {
        let page = Page::containing_address(virt_page_addr);
        map_page(page, frame_allocator.allocate_frame(), flags, p4, frame_allocator);
    }

    stack_end
}

pub(crate) fn identity_map(frame: PhysFrame, flags: PageTableFlags, p4: &mut PageTable, frame_allocator: &mut FrameAllocator) {
    let page = Page::containing_address(VirtAddr::new(frame.start_address().as_u64()));
    map_page(page, frame, flags, p4, frame_allocator);
}

pub(crate) fn map_segment(kernel_start: PhysAddr, program_header: ProgramHeader, p4: &mut PageTable,
    frame_allocator: &mut FrameAllocator)
{
    let typ = program_header.get_type().unwrap();
    match typ {
        program::Type::Load => {
            let file_offset = program_header.offset();
            let phys_start_addr = kernel_start + file_offset;
            let size = program_header.mem_size();
            let virt_start_addr = VirtAddr::new(program_header.virtual_addr());
            let virt_end_addr = virt_start_addr + size;

            let flags = program_header.flags();
            let mut page_table_flags = PageTableFlags::PRESENT;
            if !flags.is_execute() { page_table_flags |= PageTableFlags::NO_EXECUTE };
            if flags.is_write() { page_table_flags |= PageTableFlags::WRITABLE };

            for offset in (0..).step_by(usize_from(PAGE_SIZE)) {
                let page = Page::containing_address(virt_start_addr + offset);
                let frame = PhysFrame::containing_address(phys_start_addr + offset);
                if page.start_address() >= virt_end_addr {
                    break
                }
                use core::fmt::Write;
                write!(::printer::PRINTER.lock(), "{:x}->{:x} ",
                    page.start_address().as_u64(),
                    frame.start_address().as_u64()
                ).unwrap();
                map_page(page, frame, page_table_flags, p4, frame_allocator);
            }
        },
        _ => {},
    }
}

pub(crate) fn map_page(page: Page, phys_frame: PhysFrame, flags: PageTableFlags,
    p4: &mut PageTable, frame_allocator: &mut FrameAllocator)
{
    fn as_page_table_ptr(addr: PhysAddr) -> *mut PageTable {
        usize_from(addr.as_u64()) as *const PageTable as *mut PageTable
    }

    fn create_and_link_page_table(frame_allocator: &mut FrameAllocator,
        parent_table_entry: &mut PageTableEntry) -> &'static mut PageTable
    {
        let table_frame = frame_allocator.allocate_frame();
        let page_table = unsafe { &mut *as_page_table_ptr(table_frame.start_address()) };
        page_table.zero();
        parent_table_entry.set(table_frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
        page_table
    }

    fn get_or_create_next_page_table(frame_allocator: &mut FrameAllocator,
        page_table_entry: &mut PageTableEntry) -> &'static mut PageTable
    {
        match page_table_entry.points_to() {
            Some(addr) => unsafe { &mut *as_page_table_ptr(addr) },
            None => create_and_link_page_table(frame_allocator, page_table_entry)
        }
    }

    let virt_page_addr = page.start_address();

    let p4_entry = &mut p4[virt_page_addr.p4_index()];
    let p3 = get_or_create_next_page_table(frame_allocator, p4_entry);

    let p3_entry = &mut p3[virt_page_addr.p3_index()];
    let p2 = get_or_create_next_page_table(frame_allocator, p3_entry);

    let p2_entry = &mut p2[virt_page_addr.p2_index()];
    let p1 = get_or_create_next_page_table(frame_allocator, p2_entry);

    let p1_entry = &mut p1[virt_page_addr.p1_index()];
    assert!(p1_entry.is_unused(), "page for {:?} already in use", virt_page_addr);
    p1_entry.set(phys_frame, flags);
}
