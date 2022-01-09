use core::mem::align_of;

use crate::{
    binary::{level_4_entries::UsedLevel4Entries, PAGE_SIZE},
    boot_info::TlsTemplate,
};
use x86_64::{
    align_up,
    structures::paging::{
        mapper::{MappedFrame, MapperAllSizes, TranslateResult},
        FrameAllocator, Page, PageSize, PageTableFlags as Flags, PhysFrame, Size4KiB, Translate,
    },
    PhysAddr, VirtAddr,
};
use xmas_elf::{
    dynamic, header,
    program::{self, ProgramHeader, SegmentData, Type},
    sections::Rela,
    ElfFile,
};

/// Used by [`Inner::make_mut`] and [`Inner::clean_copied_flag`].
const COPIED: Flags = Flags::BIT_9;

struct Loader<'a, M, F> {
    elf_file: ElfFile<'a>,
    inner: Inner<'a, M, F>,
}

struct Inner<'a, M, F> {
    kernel_offset: PhysAddr,
    virtual_address_offset: u64,
    page_table: &'a mut M,
    frame_allocator: &'a mut F,
}

impl<'a, M, F> Loader<'a, M, F>
where
    M: MapperAllSizes + Translate,
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

        let virtual_address_offset = match elf_file.header.pt2.type_().as_type() {
            header::Type::None => unimplemented!(),
            header::Type::Relocatable => unimplemented!(),
            header::Type::Executable => 0,
            header::Type::SharedObject => 0x400000,
            header::Type::Core => unimplemented!(),
            header::Type::ProcessorSpecific(_) => unimplemented!(),
        };

        header::sanity_check(&elf_file)?;
        let loader = Loader {
            elf_file,
            inner: Inner {
                kernel_offset,
                virtual_address_offset,
                page_table,
                frame_allocator,
            },
        };

        Ok(loader)
    }

    fn load_segments(&mut self) -> Result<Option<TlsTemplate>, &'static str> {
        for program_header in self.elf_file.program_iter() {
            program::sanity_check(program_header, &self.elf_file)?;
        }

        // Load the segments into virtual memory.
        let mut tls_template = None;
        for program_header in self.elf_file.program_iter() {
            match program_header.get_type()? {
                Type::Load => self.inner.handle_load_segment(program_header)?,
                Type::Tls => {
                    if tls_template.is_none() {
                        tls_template = Some(self.inner.handle_tls_segment(program_header)?);
                    } else {
                        return Err("multiple TLS segments not supported");
                    }
                }
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

        // Apply relocations in virtual memory.
        for program_header in self.elf_file.program_iter() {
            if let Type::Dynamic = program_header.get_type()? {
                self.inner
                    .handle_dynamic_segment(program_header, &self.elf_file)?
            }
        }

        self.inner.remove_copied_flags(&self.elf_file).unwrap();

        Ok(tls_template)
    }

    fn entry_point(&self) -> VirtAddr {
        VirtAddr::new(self.elf_file.header.pt2.entry_point() + self.inner.virtual_address_offset)
    }

    fn used_level_4_entries(&self) -> UsedLevel4Entries {
        UsedLevel4Entries::new(
            self.elf_file.program_iter(),
            self.inner.virtual_address_offset,
        )
    }
}

impl<'a, M, F> Inner<'a, M, F>
where
    M: MapperAllSizes + Translate,
    F: FrameAllocator<Size4KiB>,
{
    fn handle_load_segment(&mut self, segment: ProgramHeader) -> Result<(), &'static str> {
        log::info!("Handling Segment: {:x?}", segment);

        let phys_start_addr = self.kernel_offset + segment.offset();
        let start_frame: PhysFrame = PhysFrame::containing_address(phys_start_addr);
        let end_frame: PhysFrame =
            PhysFrame::containing_address(phys_start_addr + segment.file_size() - 1u64);

        let virt_start_addr = VirtAddr::new(segment.virtual_addr()) + self.virtual_address_offset;
        let start_page: Page = Page::containing_address(virt_start_addr);

        let mut segment_flags = Flags::PRESENT;
        if !segment.flags().is_execute() {
            segment_flags |= Flags::NO_EXECUTE;
        }
        if segment.flags().is_write() {
            segment_flags |= Flags::WRITABLE;
        }

        // map all frames of the segment at the desired virtual address
        for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
            let offset = frame - start_frame;
            let page = start_page + offset;
            let flusher = unsafe {
                self.page_table
                    .map_to(page, frame, segment_flags, self.frame_allocator)
                    .map_err(|_err| "map_to failed")?
            };
            // we operate on an inactive page table, so there's no need to flush anything
            flusher.ignore();
        }

        // Handle .bss section (mem_size > file_size)
        if segment.mem_size() > segment.file_size() {
            // .bss section (or similar), which needs to be mapped and zeroed
            self.handle_bss_section(&segment, segment_flags)?;
        }

        Ok(())
    }

    fn handle_bss_section(
        &mut self,
        segment: &ProgramHeader,
        segment_flags: Flags,
    ) -> Result<(), &'static str> {
        log::info!("Mapping bss section");

        let virt_start_addr = VirtAddr::new(segment.virtual_addr()) + self.virtual_address_offset;
        let mem_size = segment.mem_size();
        let file_size = segment.file_size();

        // calculate virual memory region that must be zeroed
        let zero_start = virt_start_addr + file_size;
        let zero_end = virt_start_addr + mem_size;

        // a type alias that helps in efficiently clearing a page
        type PageArray = [u64; Size4KiB::SIZE as usize / 8];
        const ZERO_ARRAY: PageArray = [0; Size4KiB::SIZE as usize / 8];

        // In some cases, `zero_start` might not be page-aligned. This requires some
        // special treatment because we can't safely zero a frame of the original file.
        let data_bytes_before_zero = zero_start.as_u64() & 0xfff;
        if data_bytes_before_zero != 0 {
            // The last non-bss frame of the segment consists partly of data and partly of bss
            // memory, which must be zeroed. Unfortunately, the file representation might have
            // reused the part of the frame that should be zeroed to store the next segment. This
            // means that we can't simply overwrite that part with zeroes, as we might overwrite
            // other data this way.
            //
            // Example:
            //
            //   XXXXXXXXXXXXXXX000000YYYYYYY000ZZZZZZZZZZZ     virtual memory (XYZ are data)
            //   |·············|     /·····/   /·········/
            //   |·············| ___/·····/   /·········/
            //   |·············|/·····/‾‾‾   /·········/
            //   |·············||·····|/·̅·̅·̅·̅·̅·····/‾‾‾‾
            //   XXXXXXXXXXXXXXXYYYYYYYZZZZZZZZZZZ              file memory (zeros are not saved)
            //   '       '       '       '        '
            //   The areas filled with dots (`·`) indicate a mapping between virtual and file
            //   memory. We see that the data regions `X`, `Y`, `Z` have a valid mapping, while
            //   the regions that are initialized with 0 have not.
            //
            //   The ticks (`'`) below the file memory line indicate the start of a new frame. We
            //   see that the last frames of the `X` and `Y` regions in the file are followed
            //   by the bytes of the next region. So we can't zero these parts of the frame
            //   because they are needed by other memory regions.
            //
            // To solve this problem, we need to allocate a new frame for the last segment page
            // and copy all data content of the original frame over. Afterwards, we can zero
            // the remaining part of the frame since the frame is no longer shared with other
            // segments now.

            let last_page = Page::containing_address(virt_start_addr + file_size - 1u64);
            let new_frame = unsafe { self.make_mut(last_page) };
            let new_bytes_ptr = new_frame.start_address().as_u64() as *mut u8;
            unsafe {
                core::ptr::write_bytes(
                    new_bytes_ptr.add(data_bytes_before_zero as usize),
                    0,
                    (Size4KiB::SIZE - data_bytes_before_zero) as usize,
                );
            }
        }

        // map additional frames for `.bss` memory that is not present in source file
        let start_page: Page =
            Page::containing_address(VirtAddr::new(align_up(zero_start.as_u64(), Size4KiB::SIZE)));
        let end_page = Page::containing_address(zero_end);
        for page in Page::range_inclusive(start_page, end_page) {
            // allocate a new unused frame
            let frame = self.frame_allocator.allocate_frame().unwrap();

            // zero frame, utilizing identity-mapping
            let frame_ptr = frame.start_address().as_u64() as *mut PageArray;
            unsafe { frame_ptr.write(ZERO_ARRAY) };

            // map frame
            let flusher = unsafe {
                self.page_table
                    .map_to(page, frame, segment_flags, self.frame_allocator)
                    .map_err(|_err| "Failed to map new frame for bss memory")?
            };
            // we operate on an inactive page table, so we don't need to flush our changes
            flusher.ignore();
        }

        Ok(())
    }

    /// This method is intended for making the memory loaded by a Load segment mutable.
    ///
    /// All memory from a Load segment starts out by mapped to the same frames that
    /// contain the elf file. Thus writing to memory in that state will cause aliasing issues.
    /// To avoid that, we allocate a new frame, copy all bytes from the old frame to the new frame,
    /// and remap the page to the new frame. At this point the page no longer aliases the elf file
    /// and we can write to it.
    ///
    /// When we map the new frame we also set [`COPIED`] flag in the page table flags, so that
    /// we can detect if the frame has already been copied when we try to modify the page again.
    ///
    /// ## Safety
    /// - `page` should be a page mapped by a Load segment.
    ///  
    /// ## Panics
    /// Panics if the page is not mapped in `self.page_table`.
    unsafe fn make_mut(&mut self, page: Page) -> PhysFrame {
        let (frame, flags) = match self.page_table.translate(page.start_address()) {
            TranslateResult::Mapped {
                frame,
                offset: _,
                flags,
            } => (frame, flags),
            TranslateResult::NotMapped => panic!("{:?} is not mapped", page),
            TranslateResult::InvalidFrameAddress(_) => unreachable!(),
        };
        let frame = if let MappedFrame::Size4KiB(frame) = frame {
            frame
        } else {
            // We only map 4k pages.
            unreachable!()
        };

        if flags.contains(COPIED) {
            // The frame was already copied, we are free to modify it.
            return frame;
        }

        // Allocate a new frame and copy the memory, utilizing that both frames are identity mapped.
        let new_frame = self.frame_allocator.allocate_frame().unwrap();
        let frame_ptr = frame.start_address().as_u64() as *const u8;
        let new_frame_ptr = new_frame.start_address().as_u64() as *mut u8;
        unsafe {
            core::ptr::copy_nonoverlapping(frame_ptr, new_frame_ptr, Size4KiB::SIZE as usize);
        }

        // Replace the underlying frame and update the flags.
        self.page_table.unmap(page).unwrap().1.ignore();
        let new_flags = flags | COPIED;
        unsafe {
            self.page_table
                .map_to(page, new_frame, new_flags, self.frame_allocator)
                .unwrap()
                .ignore();
        }

        new_frame
    }

    /// Cleans up the custom flags set by [`Inner::make_mut`].
    fn remove_copied_flags(&mut self, elf_file: &ElfFile) -> Result<(), &'static str> {
        for program_header in elf_file.program_iter() {
            if let Type::Load = program_header.get_type()? {
                let start = self.virtual_address_offset + program_header.virtual_addr();
                let end = start + program_header.mem_size();
                let start = VirtAddr::new(start);
                let end = VirtAddr::new(end);
                let start_page = Page::containing_address(start);
                let end_page = Page::containing_address(end - 1u64);
                for page in Page::<Size4KiB>::range_inclusive(start_page, end_page) {
                    // Translate the page and get the flags.
                    let res = self.page_table.translate(page.start_address());
                    let flags = match res {
                        TranslateResult::Mapped {
                            frame: _,
                            offset: _,
                            flags,
                        } => flags,
                        TranslateResult::NotMapped | TranslateResult::InvalidFrameAddress(_) => {
                            unreachable!("has the elf file not been mapped correctly?")
                        }
                    };

                    if flags.contains(COPIED) {
                        // Remove the flag.
                        unsafe {
                            self.page_table
                                .update_flags(page, flags & !COPIED)
                                .unwrap()
                                .ignore();
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_tls_segment(&mut self, segment: ProgramHeader) -> Result<TlsTemplate, &'static str> {
        Ok(TlsTemplate {
            start_addr: segment.virtual_addr() + self.virtual_address_offset,
            mem_size: segment.mem_size(),
            file_size: segment.file_size(),
        })
    }

    fn handle_dynamic_segment(
        &mut self,
        segment: ProgramHeader,
        elf_file: &ElfFile,
    ) -> Result<(), &'static str> {
        let data = segment.get_data(elf_file)?;
        let data = if let SegmentData::Dynamic64(data) = data {
            data
        } else {
            panic!("expected Dynamic64 segment")
        };

        // Find the `Rela`, `RelaSize` and `RelaEnt` entries.
        let mut rela = None;
        let mut rela_size = None;
        let mut rela_ent = None;
        for rel in data {
            let tag = rel.get_tag()?;
            match tag {
                dynamic::Tag::Rela => {
                    let ptr = rel.get_ptr()?;
                    let prev = rela.replace(ptr);
                    if prev.is_some() {
                        return Err("Dynamic section contains more than one Rela entry");
                    }
                }
                dynamic::Tag::RelaSize => {
                    let val = rel.get_val()?;
                    let prev = rela_size.replace(val);
                    if prev.is_some() {
                        return Err("Dynamic section contains more than one RelaSize entry");
                    }
                }
                dynamic::Tag::RelaEnt => {
                    let val = rel.get_val()?;
                    let prev = rela_ent.replace(val);
                    if prev.is_some() {
                        return Err("Dynamic section contains more than one RelaEnt entry");
                    }
                }
                _ => {}
            }
        }
        let offset = if let Some(rela) = rela {
            rela
        } else {
            // The section doesn't contain any relocations.

            if rela_size.is_some() || rela_ent.is_some() {
                return Err("Rela entry is missing but RelaSize or RelaEnt have been provided");
            }

            return Ok(());
        };
        let total_size = rela_size.ok_or("RelaSize entry is missing")?;
        let entry_size = rela_ent.ok_or("RelaEnt entry is missing")?;

        // Apply the mappings.
        let entries = (total_size / entry_size) as usize;
        let rela_start = elf_file
            .input
            .as_ptr()
            .wrapping_add(offset as usize)
            .cast::<Rela<u64>>();

        // Make sure the relocations are inside the elf file.
        let rela_end = rela_start.wrapping_add(entries);
        assert!(rela_start <= rela_end);
        let file_ptr_range = elf_file.input.as_ptr_range();
        assert!(
            file_ptr_range.start <= rela_start.cast(),
            "the relocation table must start in the elf file"
        );
        assert!(
            rela_end.cast() <= file_ptr_range.end,
            "the relocation table must end in the elf file"
        );

        let relas = unsafe { core::slice::from_raw_parts(rela_start, entries) };
        for rela in relas {
            let idx = rela.get_symbol_table_index();
            assert_eq!(
                idx, 0,
                "relocations using the symbol table are not supported"
            );

            match rela.get_type() {
                // R_AMD64_RELATIVE
                8 => {
                    check_is_in_load(elf_file, rela.get_offset())?;
                    let addr = self.virtual_address_offset + rela.get_offset();
                    let value = self
                        .virtual_address_offset
                        .checked_add(rela.get_addend())
                        .unwrap();

                    let ptr = addr as *mut u64;
                    if ptr as usize % align_of::<u64>() != 0 {
                        return Err("destination of relocation is not aligned");
                    }

                    let virt_addr = VirtAddr::from_ptr(ptr);
                    let page = Page::containing_address(virt_addr);
                    let offset_in_page = virt_addr - page.start_address();

                    let new_frame = unsafe { self.make_mut(page) };
                    let phys_addr = new_frame.start_address() + offset_in_page;
                    let addr = phys_addr.as_u64() as *mut u64;
                    unsafe {
                        addr.write(value);
                    }
                }
                ty => unimplemented!("relocation type {:x} not supported", ty),
            }
        }

        Ok(())
    }
}

/// Check that the virtual offset belongs to a load segment.
fn check_is_in_load(elf_file: &ElfFile, virt_offset: u64) -> Result<(), &'static str> {
    for program_header in elf_file.program_iter() {
        if let Type::Load = program_header.get_type()? {
            if program_header.virtual_addr() <= virt_offset {
                let offset_in_segment = virt_offset - program_header.virtual_addr();
                if offset_in_segment < program_header.file_size() {
                    return Ok(());
                }
            }
        }
    }
    Err("offset is not in load segment")
}

/// Loads the kernel ELF file given in `bytes` in the given `page_table`.
///
/// Returns the kernel entry point address, it's thread local storage template (if any),
/// and a structure describing which level 4 page table entries are in use.  
pub fn load_kernel(
    bytes: &[u8],
    page_table: &mut (impl MapperAllSizes + Translate),
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(VirtAddr, Option<TlsTemplate>, UsedLevel4Entries), &'static str> {
    let mut loader = Loader::new(bytes, page_table, frame_allocator)?;
    let tls_template = loader.load_segments()?;
    let used_entries = loader.used_level_4_entries();

    Ok((loader.entry_point(), tls_template, used_entries))
}
