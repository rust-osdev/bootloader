use crate::frame_allocator::FrameAllocator;
use bootloader::bootinfo::MemoryRegionType;
use bootloader::bootinfo::TlsTemplate;
use fixedvec::FixedVec;
use x86_64::structures::paging::mapper::{MapToError, MapperFlush, UnmapError};
use x86_64::structures::paging::{
    self, Mapper, Page, PageSize, PageTableFlags, PhysFrame, RecursivePageTable, Size4KiB,
};
use x86_64::{align_up, PhysAddr, VirtAddr};
use xmas_elf::program::{self, ProgramHeader64};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryInfo {
    pub stack_end: VirtAddr,
    pub tls_segment: Option<TlsTemplate>,
}

#[derive(Debug)]
pub enum MapKernelError {
    Mapping(
        /// This field is never read, but still printed as part of the Debug output on error.
        #[allow(dead_code)]
        MapToError<Size4KiB>,
    ),
    MultipleTlsSegments,
}

impl From<MapToError<Size4KiB>> for MapKernelError {
    fn from(e: MapToError<Size4KiB>) -> Self {
        MapKernelError::Mapping(e)
    }
}

pub(crate) fn map_kernel(
    kernel_start: PhysAddr,
    stack_start: Page,
    stack_size: u64,
    segments: &FixedVec<ProgramHeader64>,
    page_table: &mut RecursivePageTable,
    frame_allocator: &mut FrameAllocator,
) -> Result<MemoryInfo, MapKernelError> {
    let mut tls_segment = None;
    for segment in segments {
        let tls = map_segment(segment, kernel_start, page_table, frame_allocator)?;
        if let Some(tls) = tls {
            if tls_segment.replace(tls).is_some() {
                return Err(MapKernelError::MultipleTlsSegments);
            }
        }
    }

    // Create a stack
    let stack_start = stack_start + 1; // Leave the first page unmapped as a 'guard page'
    let stack_end = stack_start + stack_size; // stack_size is in pages

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    let region_type = MemoryRegionType::KernelStack;

    for page in Page::range(stack_start, stack_end) {
        let frame = frame_allocator
            .allocate_frame(region_type)
            .ok_or(MapToError::FrameAllocationFailed)?;
        unsafe { map_page(page, frame, flags, page_table, frame_allocator)? }.flush();
    }

    Ok(MemoryInfo {
        stack_end: stack_end.start_address(),
        tls_segment,
    })
}

pub(crate) fn map_segment(
    segment: &ProgramHeader64,
    kernel_start: PhysAddr,
    page_table: &mut RecursivePageTable,
    frame_allocator: &mut FrameAllocator,
) -> Result<Option<TlsTemplate>, MapToError<Size4KiB>> {
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
                unsafe { map_page(page, frame, page_table_flags, page_table, frame_allocator) }
                    .unwrap_or_else(|err| {
                        panic!(
                            "failed to map segment starting at {:?}: failed to map page {:?} to frame {:?}: {:?}",
                            start_page, page, frame, err
                        )
                    })
                    .flush();
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
                    unsafe {
                        map_page(
                            temp_page.clone(),
                            new_frame,
                            page_table_flags,
                            page_table,
                            frame_allocator,
                        )?
                    }
                    .flush();

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
                            UnmapError::ParentEntryHugePage => MapToError::ParentEntryHugePage,
                            UnmapError::PageNotMapped => unreachable!(),
                            UnmapError::InvalidFrameAddress(_) => unreachable!(),
                        });
                    }

                    // unmap temp page again
                    let (new_frame, flusher) = page_table.unmap(temp_page).unwrap();
                    flusher.flush();

                    // map last page to new frame
                    unsafe {
                        map_page(
                            last_page,
                            new_frame,
                            page_table_flags,
                            page_table,
                            frame_allocator,
                        )?
                    }
                    .flush();
                }

                // Map additional frames.
                let start_page: Page = Page::containing_address(VirtAddr::new(align_up(
                    zero_start.as_u64(),
                    Size4KiB::SIZE,
                )));
                let end_page = Page::containing_address(zero_end - 1usize);
                for page in Page::range_inclusive(start_page, end_page) {
                    let frame = frame_allocator
                        .allocate_frame(MemoryRegionType::Kernel)
                        .ok_or(MapToError::FrameAllocationFailed)?;
                    unsafe {
                        map_page(page, frame, page_table_flags, page_table, frame_allocator)?
                    }
                    .flush();
                }

                // zero
                for offset in file_size..mem_size {
                    let addr = virt_start_addr + offset;
                    unsafe { addr.as_mut_ptr::<u8>().write(0) };
                }
            }

            Ok(None)
        }
        program::Type::Tls => Ok(Some(TlsTemplate {
            start_addr: segment.virtual_addr,
            mem_size: segment.mem_size,
            file_size: segment.file_size,
        })),
        _ => Ok(None),
    }
}

pub(crate) unsafe fn map_page<'a, S>(
    page: Page<S>,
    phys_frame: PhysFrame<S>,
    flags: PageTableFlags,
    page_table: &mut RecursivePageTable<'a>,
    frame_allocator: &mut FrameAllocator,
) -> Result<MapperFlush<S>, MapToError<S>>
where
    S: PageSize,
    RecursivePageTable<'a>: Mapper<S>,
{
    struct PageTableAllocator<'a, 'b: 'a>(&'a mut FrameAllocator<'b>);

    unsafe impl<'a, 'b> paging::FrameAllocator<Size4KiB> for PageTableAllocator<'a, 'b> {
        fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
            self.0.allocate_frame(MemoryRegionType::PageTable)
        }
    }

    page_table.map_to(
        page,
        phys_frame,
        flags,
        &mut PageTableAllocator(frame_allocator),
    )
}
