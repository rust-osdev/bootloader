use fixedvec::FixedVec;
use frame_allocator::FrameAllocator;
use os_bootinfo::MemoryRegionType;
use x86_64::structures::paging::{self, MapToError, RecursivePageTable, UnmapError};
use x86_64::structures::paging::{Mapper, MapperFlush, Page, PageSize, PageTableFlags, PhysFrame, Size4KiB};
use x86_64::{align_up, PhysAddr, VirtAddr};
use xmas_elf::program::{self, ProgramHeader64};

pub(crate) fn map_kernel(
    kernel_start: PhysAddr,
    segments: &FixedVec<ProgramHeader64>,
    page_table: &mut RecursivePageTable,
    frame_allocator: &mut FrameAllocator,
) -> Result<VirtAddr, MapToError> {
    for segment in segments {
        map_segment(segment, kernel_start, page_table, frame_allocator)?;
    }

    // create a stack
    // TODO create a stack range dynamically (based on where the kernel is loaded)
    let stack_start = Page::containing_address(VirtAddr::new(0x57AC_0000_0000));
    let stack_size: u64 = 512; // in pages
    let stack_end = stack_start + stack_size;

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    let region_type = MemoryRegionType::KernelStack;

    for page in Page::range(stack_start, stack_end) {
        let frame = frame_allocator
            .allocate_frame(region_type)
            .ok_or(MapToError::FrameAllocationFailed)?;
        map_page(page, frame, flags, page_table, frame_allocator)?.flush();
    }

    Ok(stack_end.start_address())
}

pub(crate) fn map_segment(
    segment: &ProgramHeader64,
    kernel_start: PhysAddr,
    page_table: &mut RecursivePageTable,
    frame_allocator: &mut FrameAllocator,
) -> Result<(), MapToError> {
    let typ = segment.get_type().unwrap();
    match typ {
        program::Type::Load => {
            let mem_size = segment.mem_size;
            let file_size = segment.file_size;
            let file_offset = segment.offset;
            let phys_start_addr = kernel_start + file_offset;
            let virt_start_addr = VirtAddr::new(segment.virtual_addr);

            let start_page: Page = Page::containing_address(virt_start_addr);
            let start_frame = PhysFrame::containing_address(phys_start_addr);
            let end_frame = PhysFrame::containing_address(phys_start_addr + file_size - 1u64);

            let flags = segment.flags;
            let mut page_table_flags = PageTableFlags::PRESENT;
            if !flags.is_execute() {
                page_table_flags |= PageTableFlags::NO_EXECUTE
            };
            if flags.is_write() {
                page_table_flags |= PageTableFlags::WRITABLE
            };

            for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
                let offset = frame - start_frame;
                let page = start_page + offset;
                map_page(page, frame, page_table_flags, page_table, frame_allocator)?.flush();
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
                    let temp_page: Page = Page::containing_address(VirtAddr::new(0xfeeefeee000));
                    let new_frame = frame_allocator
                        .allocate_frame(MemoryRegionType::Kernel)
                        .ok_or(MapToError::FrameAllocationFailed)?;
                    map_page(
                        temp_page.clone(),
                        new_frame.clone(),
                        page_table_flags,
                        page_table,
                        frame_allocator,
                    )?.flush();

                    type PageArray = [u64; Size4KiB::SIZE as usize / 8];

                    let last_page = Page::containing_address(virt_start_addr + file_size - 1u64);
                    let last_page_ptr = last_page.start_address().as_ptr::<PageArray>();
                    let temp_page_ptr = temp_page.start_address().as_mut_ptr::<PageArray>();

                    unsafe {
                        // copy contents
                        temp_page_ptr.write(last_page_ptr.read());
                    }

                    // remap last page
                    if let Err(e) = page_table.unmap(last_page.clone()) {
                        return Err(match e {
                            UnmapError::ParentEntryHugePage => {
                                MapToError::ParentEntryHugePage
                            }
                            UnmapError::PageNotMapped => unreachable!(),
                            UnmapError::InvalidFrameAddress(_) => unreachable!(),
                        });
                    }

                    map_page(
                        last_page,
                        new_frame,
                        page_table_flags,
                        page_table,
                        frame_allocator,
                    )?.flush();
                }

                // Map additional frames.
                let start_page: Page = Page::containing_address(VirtAddr::new(align_up(
                    zero_start.as_u64(),
                    Size4KiB::SIZE,
                )));
                let end_page = Page::containing_address(zero_end);
                for page in Page::range_inclusive(start_page, end_page) {
                    let frame = frame_allocator
                        .allocate_frame(MemoryRegionType::Kernel)
                        .ok_or(MapToError::FrameAllocationFailed)?;
                    map_page(page, frame, page_table_flags, page_table, frame_allocator)?.flush();
                }

                // zero
                for offset in file_size..mem_size {
                    let addr = virt_start_addr + offset;
                    unsafe { addr.as_mut_ptr::<u8>().write(0) };
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn map_page<'a, S>(
    page: Page<S>,
    phys_frame: PhysFrame<S>,
    flags: PageTableFlags,
    page_table: &mut RecursivePageTable<'a>,
    frame_allocator: &mut FrameAllocator,
) -> Result<MapperFlush<S>, MapToError>
where
    S: PageSize,
    RecursivePageTable<'a>: Mapper<S>,
{
    struct PageTableAllocator<'a, 'b: 'a>(&'a mut FrameAllocator<'b>);

    impl<'a, 'b> paging::FrameAllocator<Size4KiB> for PageTableAllocator<'a, 'b> {
        fn alloc(&mut self) -> Option<PhysFrame<Size4KiB>> {
            self.0.allocate_frame(MemoryRegionType::PageTable)
        }
    }

    page_table.map_to(page, phys_frame, flags, &mut PageTableAllocator(frame_allocator))
}
