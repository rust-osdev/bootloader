use frame_allocator::FrameAllocator;
use x86_64::{PhysAddr, VirtAddr, align_up};
use x86_64::structures::paging::{PAGE_SIZE, PageTableFlags, Page, PhysFrame};
use x86_64::structures::paging::{RecursivePageTable, MapToError, UnmapError};
use usize_conversions::usize_from;
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
    let stack_start = VirtAddr::new(0x57AC_0000_0000);
    let stack_size = 1 * 1024 * 1024;
    let stack_end = stack_start + stack_size;

    let page_size = usize_from(PAGE_SIZE);
    let virt_page_iter = (stack_start..(stack_start + stack_size)).step_by(page_size);
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    let region_type = MemoryRegionType::Kernel;


    for virt_page_addr in virt_page_iter {
        let page = Page::containing_address(virt_page_addr);
        let frame = frame_allocator.allocate_frame(region_type)
            .ok_or(MapToError::FrameAllocationFailed)?;
        map_page(page, frame, flags, page_table, frame_allocator)?;
    }

    Ok(stack_end)
}

pub(crate) fn map_segment(segment: &ProgramHeader64, kernel_start: PhysAddr,
    page_table: &mut RecursivePageTable, frame_allocator: &mut FrameAllocator)
    -> Result<(), MapToError>
{
    let typ = segment.get_type().unwrap();
    match typ {
        program::Type::Load => {
            let mem_size = segment.mem_size();
            let file_size = segment.file_size();
            let file_offset = segment.offset();
            let phys_start_addr = kernel_start + file_offset;
            let virt_start_addr = VirtAddr::new(segment.virtual_addr());

            let flags = segment.flags();
            let mut page_table_flags = PageTableFlags::PRESENT;
            if !flags.is_execute() { page_table_flags |= PageTableFlags::NO_EXECUTE };
            if flags.is_write() { page_table_flags |= PageTableFlags::WRITABLE };

            for offset in (0..align_up(file_size, u64::from(PAGE_SIZE))).step_by(usize_from(PAGE_SIZE)) {
                let page = Page::containing_address(virt_start_addr + offset);
                let frame = PhysFrame::containing_address(phys_start_addr + offset);
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

                    let last_page = Page::containing_address(virt_start_addr + file_size - 1);
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

                    map_page(last_page, new_frame, page_table_flags, page_table, frame_allocator)?;
                }

                // Map additional frames.
                let range_start = align_up(zero_start.as_u64(), u64::from(PAGE_SIZE));
                let range_end = align_up(zero_end.as_u64(), u64::from(PAGE_SIZE));
                for addr in (range_start..range_end).step_by(usize_from(PAGE_SIZE)) {
                    let page = Page::containing_address(VirtAddr::new(addr));
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
