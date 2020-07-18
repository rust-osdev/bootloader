use x86_64::{
    structures::paging::{
        FrameAllocator, MapperAllSizes, Page, PageTableFlags as Flags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};
use xmas_elf::{
    header,
    program::{self, ProgramHeader, Type},
    ElfFile,
};

const PAGE_SIZE: u64 = 4096;

struct Loader<'a, M, F> {
    elf_file: ElfFile<'a>,
    inner: Inner<'a, M, F>,
}

struct Inner<'a, M, F> {
    kernel_offset: PhysAddr,
    page_table: &'a mut M,
    frame_allocator: &'a mut F,
}

impl<'a, M, F> Loader<'a, M, F>
where
    M: MapperAllSizes,
    F: FrameAllocator<Size4KiB>,
{
    fn new(
        bytes: &'a [u8],
        page_table: &'a mut M,
        frame_allocator: &'a mut F,
    ) -> Result<Self, &'static str> {
        log::info!("Elf file loaded at {:#p}", bytes);
        let kernel_offset = PhysAddr::new(&bytes[0] as *const u8 as u64);
        if !kernel_offset.is_aligned(PAGE_SIZE) {
            return Err("Loaded kernel ELF file is not sufficiently aligned");
        }

        let elf_file = ElfFile::new(bytes)?;
        header::sanity_check(&elf_file)?;
        let loader = Loader {
            elf_file,
            inner: Inner {
                kernel_offset,
                page_table,
                frame_allocator,
            },
        };

        Ok(loader)
    }

    fn load_segments(&mut self) -> Result<(), &'static str> {
        for program_header in self.elf_file.program_iter() {
            program::sanity_check(program_header, &self.elf_file)?;
            match program_header.get_type()? {
                Type::Load => self.inner.handle_load_segment(program_header)?,
                Type::Tls => self.inner.handle_tls_segment(program_header)?,
                Type::Null
                | Type::Dynamic
                | Type::Interp
                | Type::Note
                | Type::ShLib
                | Type::Phdr
                | Type::GnuRelro
                | Type::OsSpecific(_)
                | Type::ProcessorSpecific(_) => {}
            }
        }
        Ok(())
    }
}

impl<'a, M, F> Inner<'a, M, F>
where
    M: MapperAllSizes,
    F: FrameAllocator<Size4KiB>,
{
    fn handle_load_segment(&mut self, segment: ProgramHeader) -> Result<(), &'static str> {
        log::info!("Handling Segment: {:x?}", segment);

        let phys_start_addr = self.kernel_offset + segment.offset();
        let start_frame: PhysFrame = PhysFrame::containing_address(phys_start_addr);
        let end_frame: PhysFrame =
            PhysFrame::containing_address(phys_start_addr + segment.file_size() - 1u64);

        let virt_start_addr = VirtAddr::new(segment.virtual_addr());
        let start_page: Page = Page::containing_address(virt_start_addr);

        let mut flags = Flags::PRESENT;
        if !segment.flags().is_execute() {
            flags |= Flags::NO_EXECUTE;
        }
        if segment.flags().is_write() {
            flags |= Flags::WRITABLE;
        }

        for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
            let offset = frame - start_frame;
            let page = start_page + offset;
            unsafe {
                self.page_table
                    .map_to(page, frame, flags, self.frame_allocator)
                    .map_err(|_err| "map_to failed")?
            }
            .flush();
        }

        Ok(())
    }

    fn handle_tls_segment(&self, segment: ProgramHeader) -> Result<(), &'static str> {
        todo!()
    }
}

pub fn load_kernel(
    bytes: &[u8],
    page_table: &mut impl MapperAllSizes,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), &'static str> {
    let mut loader = Loader::new(bytes, page_table, frame_allocator)?;
    loader.load_segments()?;

    Err("unfinished implementation!")
}
