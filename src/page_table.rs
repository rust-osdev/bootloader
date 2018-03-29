use frame_allocator::FrameAllocator;
use x86_64::{PhysAddr, VirtAddr, align_up};
use x86_64::structures::paging::{PAGE_SIZE, PageTable, PageTableFlags, PageTableEntry, Page, PhysFrame};
use usize_conversions::usize_from;
use xmas_elf::program::{self, ProgramHeader64};
use fixedvec::FixedVec;

pub(crate) fn map_kernel(kernel_start: PhysAddr, segments: &FixedVec<ProgramHeader64>,
    p4: &mut PageTable, frame_allocator: &mut FrameAllocator) -> VirtAddr
{
    for segment in segments {
        map_segment(segment, kernel_start, p4, frame_allocator);
    }

    // create a stack
    // TODO create a stack range dynamically (based on where the kernel is loaded)
    let stack_start = VirtAddr::new(0x57AC_0000_0000);
    let stack_size: u64 = 1 * 1024 * 1024;
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

pub(crate) fn map_segment(segment: &ProgramHeader64, kernel_start: PhysAddr, p4: &mut PageTable,
    frame_allocator: &mut FrameAllocator)
{
    unsafe fn zero_frame(frame: &PhysFrame) {
        let frame_ptr = frame.start_address().as_u64() as *mut [u8; PAGE_SIZE as usize];
        *frame_ptr = [0; PAGE_SIZE as usize];
    }

    let typ = segment.get_type().unwrap();
    match typ {
        program::Type::Load => {
            let mem_size = segment.mem_size;
            let file_size = segment.file_size;
            let file_offset = segment.offset;
            let phys_start_addr = kernel_start + file_offset;
            let virt_start_addr = VirtAddr::new(segment.virtual_addr);

            let flags = segment.flags;
            let mut page_table_flags = PageTableFlags::PRESENT;
            if !flags.is_execute() { page_table_flags |= PageTableFlags::NO_EXECUTE };
            if flags.is_write() { page_table_flags |= PageTableFlags::WRITABLE };

            for offset in (0..).step_by(usize_from(PAGE_SIZE)) {
                let page = Page::containing_address(virt_start_addr + offset);
                let frame;
                if offset >= mem_size {
                    break
                }
                if offset >= file_size {
                    // map to zeroed frame
                    frame = frame_allocator.allocate_frame();
                    unsafe { zero_frame(&frame) };
                } else if align_up(offset, u64::from(PAGE_SIZE)) - 1 >= file_size {
                    // part of the page should be zeroed
                    frame = frame_allocator.allocate_frame();
                    unsafe { zero_frame(&frame) };
                    // copy data from kernel image
                    let data_frame_start = PhysFrame::containing_address(phys_start_addr + offset);
                    let data_ptr = data_frame_start.start_address().as_u64() as *const u8;
                    let frame_ptr = frame.start_address().as_u64() as *mut u8;
                    for i in offset..file_size {
                        let i = i as isize;
                        unsafe { frame_ptr.offset(i).write(data_ptr.offset(i).read()) };
                    }
                } else {
                    // map to part of kernel binary
                    frame = PhysFrame::containing_address(phys_start_addr + offset);
                }
                map_page(page, frame, page_table_flags, p4, frame_allocator);
            }
        },
        _ => {},
    }
}

pub(crate) fn map_page(page: Page, phys_frame: PhysFrame, flags: PageTableFlags,
    p4: &mut PageTable, frame_allocator: &mut FrameAllocator)
{
    fn as_page_table_ptr(frame: &PhysFrame) -> *mut PageTable {
        usize_from(frame.start_address().as_u64()) as *const PageTable as *mut PageTable
    }

    fn create_and_link_page_table(frame_allocator: &mut FrameAllocator,
        parent_table_entry: &mut PageTableEntry) -> &'static mut PageTable
    {
        let table_frame = frame_allocator.allocate_frame();
        let page_table = unsafe { &mut *as_page_table_ptr(&table_frame) };
        page_table.zero();
        parent_table_entry.set(table_frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
        page_table
    }

    fn get_or_create_next_page_table(frame_allocator: &mut FrameAllocator,
        page_table_entry: &mut PageTableEntry) -> &'static mut PageTable
    {
        match page_table_entry.frame() {
            Some(frame) => unsafe { &mut *as_page_table_ptr(&frame) },
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
