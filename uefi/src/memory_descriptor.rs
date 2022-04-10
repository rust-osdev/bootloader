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

    fn on_bootloader_exit(&mut self) {
        match self.0.ty {
            // the bootloader is about to exit, so we can reallocate its data
            MemoryType::LOADER_CODE
            | MemoryType::LOADER_DATA
            | MemoryType::BOOT_SERVICES_CODE
            | MemoryType::BOOT_SERVICES_DATA
            | MemoryType::RUNTIME_SERVICES_CODE
            | MemoryType::RUNTIME_SERVICES_DATA => {
                // we don't need this data anymore
                self.0.ty = MemoryType::CONVENTIONAL;
            }
            _ => {}
        }
    }
}
