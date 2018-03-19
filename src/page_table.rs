use frame_allocator::FrameAllocator;
use x86_64::{PhysAddr, VirtAddr, align_up};
use x86_64::structures::paging::{PAGE_SIZE, PageTableFlags, Page, PhysFrame};
use x86_64::structures::paging::{RecursivePageTable, MapToError, UnmapError};
use x86_64::instructions::tlb;
use xmas_elf::program::{self, ProgramHeader64};
use fixedvec::FixedVec;
use os_bootinfo::MemoryRegionType;

pub(crate) fn map_kernel(kernel_start: PhysAddr, segments: &FixedVec<ProgramHeader64>,
    page_table: &mut RecursivePageTable, frame_allocator: &mut FrameAllocator)
    -> Result<VirtAddr, MapToError>
{
    for segment in segments {
        map_segment(segment, kernel_start, page_table, frame_allocator)?;
    }

    // create a stack
    // TODO create a stack range dynamically (based on where the kernel is loaded)
    let stack_start = Page::containing_address(VirtAddr::new(0x57AC_0000_0000));
    let stack_size: u64 = 256; // in pages
    let stack_end = stack_start + stack_size;

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    let region_type = MemoryRegionType::Kernel;

    for page in Page::range(stack_start, stack_end) {
        let frame = frame_allocator.allocate_frame(region_type)
            .ok_or(MapToError::FrameAllocationFailed)?;
        map_page(page, frame, flags, page_table, frame_allocator)?;
    }

    Ok(stack_end.start_address())
}

pub(crate) fn map_segment(segment: &ProgramHeader64, kernel_start: PhysAddr,
    page_table: &mut RecursivePageTable, frame_allocator: &mut FrameAllocator)
    -> Result<(), MapToError>
{
    let typ = segment.get_type().unwrap();
    match typ {
        program::Type::Load => {
            let mem_size = segment.mem_size;
            let file_size = segment.file_size;
            let file_offset = segment.offset;
            let phys_start_addr = kernel_start + file_offset;
            let virt_start_addr = VirtAddr::new(segment.virtual_addr);

            let start_page = Page::containing_address(virt_start_addr);
            let start_frame = PhysFrame::containing_address(phys_start_addr);
            let end_frame = PhysFrame::containing_address(phys_start_addr + file_size - 1u64);

            let flags = segment.flags;
            let mut page_table_flags = PageTableFlags::PRESENT;
            if !flags.is_execute() { page_table_flags |= PageTableFlags::NO_EXECUTE };
            if flags.is_write() { page_table_flags |= PageTableFlags::WRITABLE };

            for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
                let offset = frame - start_frame;
                let page = start_page + offset;
                map_page(page, frame, page_table_flags, page_table, frame_allocator)?;
            }

            if mem_size > file_size {
                // .bss section (or similar), which needs to be zeroed
                let zero_start = virt_start_addr + file_size;
                let zero_end = virt_start_addr + mem_size;
                if zero_start.as_u64() & 0xfff != 0 {
                    // A part of the last mapped frame needs to be zeroed. This is
                    // not possible since it could already contains parts of the next
                    // segment. Thus, we need to copy it before zeroing.

                    // TODO: search for a free page dynamically
                    let temp_page = Page::containing_address(VirtAddr::new(0xfeeefeee000));
                    let new_frame = frame_allocator.allocate_frame(MemoryRegionType::Kernel)
                        .ok_or(MapToError::FrameAllocationFailed)?;
                    map_page(temp_page.clone(), new_frame.clone(), page_table_flags, page_table,
                        frame_allocator)?;

                    type PageArray = [u64; PAGE_SIZE as usize / 8];

                    let last_page = Page::containing_address(virt_start_addr + file_size - 1u64);
                    let last_page_ptr = last_page.start_address().as_ptr::<PageArray>();
                    let temp_page_ptr = temp_page.start_address().as_mut_ptr::<PageArray>();

                    unsafe {
                        // copy contents
                        temp_page_ptr.write(last_page_ptr.read());
                    }

                    // remap last page
                    if let Err(e) = page_table.unmap(last_page.clone(), &mut |_| {}) {
                        return Err(match e {
                            UnmapError::EntryWithInvalidFlagsPresent(_, _) => {
                                MapToError::EntryWithInvalidFlagsPresent
                            },
                            UnmapError::PageNotMapped(_) => unreachable!(),
                        });
                    }
                    tlb::flush(last_page.start_address());

                    map_page(last_page, new_frame, page_table_flags, page_table, frame_allocator)?;
                }

                // Map additional frames.
                let start_page = Page::containing_address(
                    VirtAddr::new(align_up(zero_start.as_u64(), u64::from(PAGE_SIZE)))
                );
                let end_page = Page::containing_address(zero_end);
                for page in Page::range_inclusive(start_page, end_page) {
                    let frame = frame_allocator.allocate_frame(MemoryRegionType::Kernel)
                        .ok_or(MapToError::FrameAllocationFailed)?;
                    map_page(page, frame, page_table_flags, page_table, frame_allocator)?;
                }

                // zero
                for offset in file_size..mem_size {
                    let addr = virt_start_addr + offset;
                    unsafe { addr.as_mut_ptr::<u8>().write(0) };
                }
            }
        },
        _ => {},
    }
    Ok(())
}

pub(crate) fn map_page(page: Page, phys_frame: PhysFrame, flags: PageTableFlags,
        page_table: &mut RecursivePageTable, frame_allocator: &mut FrameAllocator
    ) -> Result<(), MapToError>
{
    page_table.map_to(page, phys_frame, flags, &mut || {
        frame_allocator.allocate_frame(MemoryRegionType::PageTable)
    })
}
