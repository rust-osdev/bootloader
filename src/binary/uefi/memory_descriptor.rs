use crate::binary::legacy_memory_region::LegacyMemoryRegion;
use uefi::table::boot::{MemoryDescriptor, MemoryType};
use x86_64::PhysAddr;

const PAGE_SIZE: u64 = 4096;

impl<'a> LegacyMemoryRegion for MemoryDescriptor {
    fn start(&self) -> PhysAddr {
        PhysAddr::new(self.phys_start)
    }

    fn len(&self) -> u64 {
        self.page_count * PAGE_SIZE
    }

    fn usable(&self) -> bool {
        self.ty == MemoryType::CONVENTIONAL
    }

    fn set_start(&mut self, new_start: PhysAddr) {
        self.phys_start = new_start.as_u64();
    }
}
