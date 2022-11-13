use bootloader_api::info::MemoryRegionKind;
use bootloader_x86_64_bios_common::E820MemoryRegion;
use bootloader_x86_64_common::legacy_memory_region::LegacyMemoryRegion;
use x86_64::PhysAddr;

impl LegacyMemoryRegion for MemoryRegion {
    fn start(&self) -> PhysAddr {
        PhysAddr::new(self.0.start_addr)
    }

    fn len(&self) -> u64 {
        self.0.len
    }

    fn kind(&self) -> MemoryRegionKind {
        match self.0.region_type {
            1 => MemoryRegionKind::Usable,
            other => MemoryRegionKind::UnknownBios(other),
        }
    }

    fn usable_after_bootloader_exit(&self) -> bool {
        matches!(self.kind(), MemoryRegionKind::Usable)
    }
}

/// A physical memory region returned by an `e820` BIOS call.
///
/// See http://wiki.osdev.org/Detecting_Memory_(x86)#Getting_an_E820_Memory_Map for more info.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct MemoryRegion(pub E820MemoryRegion);
