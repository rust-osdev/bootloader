use x86_64::{
    structures::paging::{Page, PhysFrame},
    PhysAddr, VirtAddr,
};
use xmas_elf::{
    header,
    program::{self, ProgramHeader, Type},
    ElfFile,
};

const PAGE_SIZE: u64 = 4096;

struct Loader<'a> {
    bytes: &'a [u8],
    elf_file: ElfFile<'a>,
}

impl<'a> Loader<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self, &'static str> {
        let elf_file = ElfFile::new(bytes)?;
        header::sanity_check(&elf_file)?;
        let loader = Loader { bytes, elf_file };

        log::info!("Elf file loaded at {:#p}", bytes);
        if !loader.kernel_offset().is_aligned(PAGE_SIZE) {
            return Err("Loaded kernel ELF file is not sufficiently aligned");
        }

        Ok(loader)
    }

    fn kernel_offset(&self) -> PhysAddr {
        PhysAddr::new(&self.bytes[0] as *const u8 as u64)
    }

    fn load_segments(&self) -> Result<(), &'static str> {
        for program_header in self.elf_file.program_iter() {
            program::sanity_check(program_header, &self.elf_file)?;
            match program_header.get_type()? {
                Type::Load => self.handle_load_segment(program_header)?,
                Type::Tls => self.handle_tls_segment(program_header)?,
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

    fn handle_load_segment(&self, segment: ProgramHeader) -> Result<(), &'static str> {
        log::info!("Handling Segment: {:x?}", segment);

        let phys_start_addr = self.kernel_offset() + segment.offset();
        let start_frame: PhysFrame = PhysFrame::containing_address(phys_start_addr);
        let end_frame: PhysFrame =
            PhysFrame::containing_address(phys_start_addr + segment.file_size() - 1u64);

        let virt_start_addr = VirtAddr::new(segment.virtual_addr());
        let start_page: Page = Page::containing_address(virt_start_addr);

        for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
            let offset = frame - start_frame;
            let page = start_page + offset;
            log::info!("mapping {:?} to {:?}", page, frame);
        }

        Ok(())
    }

    fn handle_tls_segment(&self, segment: ProgramHeader) -> Result<(), &'static str> {
        todo!()
    }
}

pub fn load_kernel(bytes: &[u8]) -> Result<(), &'static str> {
    let loader = Loader::new(bytes)?;
    loader.load_segments()?;

    Err("unfinished implementation!")
}
