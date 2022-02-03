use core::{alloc::Layout, convert::TryInto};
use usize_conversions::IntoUsize;
use x86_64::{
    structures::paging::{Page, PageTableIndex, Size4KiB},
    PhysAddr, VirtAddr,
};
use xmas_elf::program::ProgramHeader;

use crate::{
    binary::{MemoryRegion, CONFIG},
    BootInfo,
};

/// Keeps track of used entries in a level 4 page table.
///
/// Useful for determining a free virtual memory block, e.g. for mapping additional data.
pub struct UsedLevel4Entries {
    entry_state: [bool; 512], // whether an entry is in use by the kernel
}

impl UsedLevel4Entries {
    /// Initializes a new instance.
    ///
    /// Marks the statically configured virtual address ranges from the config as used.
    pub fn new(max_phys_addr: PhysAddr, regions_len: usize, framebuffer_size: usize) -> Self {
        let mut used = UsedLevel4Entries {
            entry_state: [false; 512],
        };

        used.entry_state[0] = true; // TODO: Can we do this dynamically?

        // Mark the statically configured ranges from the config as used.

        if let Some(physical_memory_offset) = CONFIG.physical_memory_offset {
            used.mark_range_as_used(physical_memory_offset, max_phys_addr.as_u64().into_usize());
        }

        if CONFIG.map_page_table_recursively {
            if let Some(recursive_index) = CONFIG.recursive_index {
                used.mark_p4_index_as_used(PageTableIndex::new(recursive_index));
            }
        }

        if let Some(kernel_stack_address) = CONFIG.kernel_stack_address {
            used.mark_range_as_used(kernel_stack_address, CONFIG.kernel_stack_size());
        }

        if let Some(boot_info_address) = CONFIG.boot_info_address {
            let boot_info_layout = Layout::new::<BootInfo>();
            let regions = regions_len + 1; // one region might be split into used/unused
            let memory_regions_layout = Layout::array::<MemoryRegion>(regions).unwrap();
            let (combined, _) = boot_info_layout.extend(memory_regions_layout).unwrap();

            used.mark_range_as_used(boot_info_address, combined.size());
        }

        if CONFIG.map_framebuffer {
            if let Some(framebuffer_address) = CONFIG.framebuffer_address {
                used.mark_range_as_used(framebuffer_address, framebuffer_size);
            }
        }

        used
    }

    /// Mark all p4 entries in the range `[address..address+size)` as used.
    ///
    /// `size` can be a `u64` or `usize`.
    fn mark_range_as_used<S>(&mut self, address: u64, size: S)
    where
        VirtAddr: core::ops::Add<S, Output = VirtAddr>,
    {
        let start = VirtAddr::new(address);
        let end_inclusive = (start + size) - 1usize;
        let start_page = Page::<Size4KiB>::containing_address(start);
        let end_page_inclusive = Page::<Size4KiB>::containing_address(end_inclusive);

        for p4_index in u16::from(start_page.p4_index())..=u16::from(end_page_inclusive.p4_index())
        {
            self.mark_p4_index_as_used(PageTableIndex::new(p4_index));
        }
    }

    fn mark_p4_index_as_used(&mut self, p4_index: PageTableIndex) {
        self.entry_state[usize::from(p4_index)] = true;
    }

    /// Marks the virtual address range of all segments as used.
    pub fn mark_segments<'a>(
        &mut self,
        segments: impl Iterator<Item = ProgramHeader<'a>>,
        virtual_address_offset: u64,
    ) {
        for segment in segments.filter(|s| s.mem_size() > 0) {
            self.mark_range_as_used(
                segment.virtual_addr() + virtual_address_offset,
                segment.mem_size(),
            );
        }
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
