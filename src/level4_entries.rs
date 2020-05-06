use core::convert::TryInto;
use fixedvec::FixedVec;
use x86_64::{
    structures::paging::{Page, PageTableIndex},
    VirtAddr,
};
use xmas_elf::program::ProgramHeader64;

pub struct UsedLevel4Entries {
    entry_state: [bool; 512], // whether an entry is in use by the kernel
}

impl UsedLevel4Entries {
    pub fn new(segments: &FixedVec<ProgramHeader64>) -> Self {
        let mut used = UsedLevel4Entries {
            entry_state: [false; 512],
        };

        used.entry_state[0] = true; // TODO: Can we do this dynamically?

        for segment in segments {
            let start_page: Page = Page::containing_address(VirtAddr::new(segment.virtual_addr));
            let end_page: Page =
                Page::containing_address(VirtAddr::new(segment.virtual_addr + segment.mem_size));

            for p4_index in u64::from(start_page.p4_index())..=u64::from(end_page.p4_index()) {
                used.entry_state[p4_index as usize] = true;
            }
        }

        used
    }

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
}
