use os_bootinfo::{MemoryMap, MemoryRegion, MemoryRegionType};
use x86_64::structures::paging::{PhysFrame, PAGE_SIZE};

pub(crate) struct FrameAllocator<'a> {
    pub memory_map: &'a mut MemoryMap,
}

impl<'a> FrameAllocator<'a> {
    pub(crate) fn allocate_frame(&mut self, region_type: MemoryRegionType) -> Option<PhysFrame> {
        let page_size = u64::from(PAGE_SIZE);
        let mut frame = None;
        for region in self.memory_map.iter_mut() {
            if region.region_type != MemoryRegionType::Usable {
                continue;
            }
            if region.len < page_size {
                continue;
            }
            assert_eq!(
                0,
                region.start_addr.as_u64() & 0xfff,
                "Region start address is not page aligned: {:?}",
                region
            );

            frame = Some(PhysFrame::containing_address(region.start_addr));
            region.start_addr += page_size;
            region.len -= page_size;
            break;
        }
        if let Some(frame) = frame {
            self.add_region(MemoryRegion {
                start_addr: frame.start_address(),
                len: page_size,
                region_type,
            });
            Some(frame)
        } else {
            None
        }
    }

    pub(crate) fn deallocate_frame(&mut self, frame: PhysFrame) {
        let page_size = u64::from(PAGE_SIZE);
        self.add_region_overwrite(
            MemoryRegion {
                start_addr: frame.start_address(),
                len: page_size,
                region_type: MemoryRegionType::Usable,
            },
            true,
        );
    }

    /// Adds the passed region to the memory map.
    ///
    /// This function automatically adjusts the existing regions so that no overlap occurs.
    ///
    /// Panics if a non-usable region (e.g. a reserved region) overlaps with the passed region.
    pub(crate) fn add_region(&mut self, region: MemoryRegion) {
        self.add_region_overwrite(region, false);
    }

    fn add_region_overwrite(&mut self, region: MemoryRegion, overwrite: bool) {
        assert_eq!(
            0,
            region.start_addr.as_u64() & 0xfff,
            "Region start address is not page aligned: {:?}",
            region
        );

        let mut region_already_inserted = false;
        let mut split_region = None;

        for r in self.memory_map.iter_mut() {
            // check if region overlaps with another region
            if r.start_addr() < region.end_addr() && r.end_addr() > region.start_addr() {
                // region overlaps with `r`
                match r.region_type {
                    MemoryRegionType::Usable => {
                        if region.region_type == MemoryRegionType::Usable {
                            panic!(
                                "region {:?} overlaps with other usable region {:?}",
                                region, r
                            )
                        }
                    }
                    MemoryRegionType::InUse => {}
                    MemoryRegionType::Bootloader
                    | MemoryRegionType::Kernel
                    | MemoryRegionType::PageTable if overwrite => {}
                    _ => {
                        panic!("can't override region {:?} with {:?}", r, region);
                    }
                }
                if r.start_addr() < region.start_addr() && r.end_addr() > region.end_addr() {
                    // Case: (r = `r`, R = `region`)
                    // ----rrrrrrrrrrr----
                    // ------RRRR---------
                    r.len = region.start_addr() - r.start_addr();
                    assert!(
                        split_region.is_none(),
                        "area overlaps with multiple regions"
                    );
                    split_region = Some(MemoryRegion {
                        start_addr: region.end_addr(),
                        len: r.end_addr() - region.end_addr(),
                        region_type: r.region_type,
                    });
                } else if region.start_addr() <= r.start_addr() {
                    // Case: (r = `r`, R = `region`)
                    // ----rrrrrrrrrrr----
                    // --RRRR-------------
                    r.len -= region.end_addr() - r.start_addr();
                    r.start_addr = region.end_addr();
                } else if region.end_addr() >= r.end_addr() {
                    // Case: (r = `r`, R = `region`)
                    // ----rrrrrrrrrrr----
                    // -------------RRRR--
                    r.len = region.start_addr() - r.start_addr();
                } else {
                    unreachable!("region overlaps in an unexpected way")
                }
            }
            // check if region is adjacent to already existing region (only if same type)
            if r.region_type == region.region_type {
                if region.end_addr() == r.start_addr() {
                    // Case: (r = `r`, R = `region`)
                    // ------rrrrrrrrrrr--
                    // --RRRR-------------
                    // => merge regions
                    r.start_addr = region.start_addr();
                    r.len += region.len;
                    region_already_inserted = true;
                } else if region.start_addr() == r.end_addr() {
                    // Case: (r = `r`, R = `region`)
                    // --rrrrrrrrrrr------
                    // -------------RRRR--
                    // => merge regions
                    r.len += region.len;
                    region_already_inserted = true;
                }
            }
        }

        if let Some(split_region) = split_region {
            self.memory_map.add_region(split_region);
        }
        if !region_already_inserted {
            self.memory_map.add_region(region);
        }
    }
}
