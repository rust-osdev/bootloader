#[derive(Debug, Eq, PartialEq)]
pub struct MemoryMap {
    regions: &'static mut [MemoryRegion],
    next_free: usize,
}

impl MemoryMap {
    pub fn new(regions: &'static mut [MemoryRegion]) -> Self {
        Self {
            regions,
            next_free: 0,
        }
    }

    pub fn add_region(&mut self, new_region: MemoryRegion) -> Result<(), ()> {
        let region = self.regions.get_mut(self.next_free).ok_or(())?;
        if *region == MemoryRegion::empty() {
            *region = new_region;
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn as_slice(&self) -> &[MemoryRegion] {
        &self.regions[..self.next_free]
    }

    pub fn as_mut_slice(&mut self) -> &mut [MemoryRegion] {
        &mut self.regions[..self.next_free]
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MemoryRegion {
    pub start: u64,
    pub end: u64,
    pub kind: MemoryRegionKind,
}

impl MemoryRegion {
    pub const fn empty() -> Self {
        MemoryRegion {
            start: 0,
            end: 0,
            kind: MemoryRegionKind::Bootloader,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MemoryRegionKind {
    Usable,
    Reserved,
    Bootloader,
}

extern "C" fn _assert_ffi(_memory_map: MemoryMap) {}
