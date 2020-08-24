use x86_64::PhysAddr;
use crate::binary::legacy_memory_region::LegacyMemoryRegion;

impl LegacyMemoryRegion for E820MemoryRegion {
    fn start(&self) -> PhysAddr {
        PhysAddr::new(self.start_addr)
    }

    fn len(&self) -> u64 {
        self.len
    }

    fn usable(&self) -> bool {
        self.region_type == 1
    }

    fn set_start(&mut self, new_start: PhysAddr) {
        self.start_addr = new_start.as_u64();
    }
}


#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct E820MemoryRegion {
    pub start_addr: u64,
    pub len: u64,
    pub region_type: u32,
    pub acpi_extended_attributes: u32,
}

/*
impl From<E820MemoryRegion> for MemoryRegion {
    fn from(region: E820MemoryRegion) -> MemoryRegion {
        let region_type = match region.region_type {
            1 => MemoryRegionType::Usable,
            2 => MemoryRegionType::Reserved,
            3 => MemoryRegionType::AcpiReclaimable,
            4 => MemoryRegionType::AcpiNvs,
            5 => MemoryRegionType::BadMemory,
            t => panic!("invalid region type {}", t),
        };
        MemoryRegion {
            range: FrameRange::new(region.start_addr, region.start_addr + region.len),
            region_type,
        }
    }
}
*/