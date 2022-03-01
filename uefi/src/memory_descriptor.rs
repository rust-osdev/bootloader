use bootloader_api::info::MemoryRegionKind;
use bootloader_x86_64_common::legacy_memory_region::LegacyMemoryRegion;
use uefi::table::boot::{MemoryDescriptor, MemoryType};
use x86_64::PhysAddr;

#[derive(Debug, Copy, Clone)]
pub struct UefiMemoryDescriptor(pub MemoryDescriptor);

const PAGE_SIZE: u64 = 4096;

impl<'a> LegacyMemoryRegion for UefiMemoryDescriptor {
    fn start(&self) -> PhysAddr {
        PhysAddr::new(self.0.phys_start)
    }

    fn len(&self) -> u64 {
        self.0.page_count * PAGE_SIZE
    }

    fn kind(&self) -> MemoryRegionKind {
        match self.0.ty {
            MemoryType::CONVENTIONAL => MemoryRegionKind::Usable,
            other => MemoryRegionKind::UnknownUefi(other.0),
        }
    }
}
