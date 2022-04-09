use core::convert::TryInto;
use x86_64::{
    structures::paging::{Page, PageTableIndex},
    VirtAddr,
};
use xmas_elf::program::ProgramHeader;

/// Keeps track of used entries in a level 4 page table.
///
/// Useful for determining a free virtual memory block, e.g. for mapping additional data.
pub struct UsedLevel4Entries {
    entry_state: [bool; 512], // whether an entry is in use by the kernel
}

impl UsedLevel4Entries {
    /// Initializes a new instance from the given ELF program segments.
    ///
    /// Marks the virtual address range of all segments as used.
    pub fn new<'a>(
        segments: impl Iterator<Item = ProgramHeader<'a>>,
        virtual_address_offset: u64,
    ) -> Self {
        let mut used = UsedLevel4Entries {
            entry_state: [false; 512],
        };

        used.entry_state[0] = true; // TODO: Can we do this dynamically?

        for segment in segments {
            let start_page: Page = Page::containing_address(VirtAddr::new(
                segment.virtual_addr() + virtual_address_offset,
            ));
            let end_page: Page = Page::containing_address(VirtAddr::new(
                segment.virtual_addr() + virtual_address_offset + segment.mem_size(),
            ));

            for p4_index in u64::from(start_page.p4_index())..=u64::from(end_page.p4_index()) {
                used.entry_state[p4_index as usize] = true;
            }
        }

        used
    }

    /// Returns a unused level 4 entry and marks it as used.
    ///
    /// Since this method marks each returned index as used, it can be used multiple times
    /// to determine multiple unused virtual memory regions.
    pub fn get_free_entry(&mut self) -> PageTableIndex {
        let (idx, entry) = self
            .entry_state
            .iter_mut()
            .enumerate()
            .find(|(_, &mut entry)| entry == false)
            .expect("no usable level 4 entries found");

        *entry = true;
        PageTableIndex::new(idx.try_into().unwrap())
    }

    /// Returns the virtual start address of an unused level 4 entry and marks it as used.
    ///
    /// This is a convenience method around [`get_free_entry`], so all of its docs applies here
    /// too.
    pub fn get_free_address(&mut self) -> VirtAddr {
        Page::from_page_table_indices_1gib(self.get_free_entry(), PageTableIndex::new(0))
            .start_address()
    }
}
