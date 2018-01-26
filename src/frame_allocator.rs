use x86_64::PhysAddr;
use x86_64::structures::paging::{PAGE_SIZE, PhysFrame};
use os_bootinfo::{BootInfo, MemoryRegion, MemoryRegionType};

pub(crate) struct FrameAllocator<'a>(pub &'a mut BootInfo);

impl<'a> FrameAllocator<'a> {
    pub(crate) fn allocate_frame(&mut self) -> PhysFrame {
        let page_size = u64::from(PAGE_SIZE);
        let mut frame = None;
        for region in &mut self.0.memory_map {
            if region.start_addr < PhysAddr::new(1024 * 1024) {
                // don't allocate memory below 1M, since it might be used by bootloader or BIOS
                continue;
            }
            if region.region_type != MemoryRegionType::Usable {
                continue;
            }
            if region.len < page_size {
                continue;
            }
            assert_eq!(0, region.start_addr.as_u64() & 0xfff,
                "Region start address must be aligned");

            frame = Some(PhysFrame::containing_address(region.start_addr));
            region.start_addr += page_size;
            break;
        }
        if let Some(frame) = frame {
            self.mark_as_used(frame.start_address(), page_size);
            frame
        } else {
            panic!("Out of physical memory");
        }
    }

    pub(crate) fn mark_as_used(&mut self, addr: PhysAddr, len: u64) {
        let mut used_region = Some(MemoryRegion {
            start_addr: addr,
            len,
            region_type: MemoryRegionType::InUse,
        });
        let mut new_region = None;

        // check if it overlaps with another region
        for region in &mut self.0.memory_map {
            let region_start = region.start_addr;
            let region_end = region.start_addr + region.len;
            let used_start = addr;
            let used_end = addr + len;

            if region_start < used_end && region_end > used_start {
                // used area overlaps with region
                assert!(region.region_type == MemoryRegionType::Usable);
                if region_start < used_start && region_end > used_end {
                    // Case: (R = region, U = used_area)
                    // ----RRRRRRRRRRR----
                    // ------UUUU---------
                    region.len = used_start - region_start;
                    assert!(new_region.is_none(), "area overlaps with multiple regions");
                    new_region = Some(MemoryRegion {
                        start_addr: used_end,
                        len: region_end - used_end,
                        region_type: MemoryRegionType::Usable,
                    });
                } else if used_start <= region_start {
                    // Case: (R = region, U = used_area)
                    // ----RRRRRRRRRRR----
                    // --UUUU-------------
                        region.start_addr = used_end;
                } else if used_end >= used_end {
                    // Case: (R = region, U = used_area)
                    // ----RRRRRRRRRRR----
                    // -------------UUUU--
                    region.len = used_start - region_start;
                }
            }
            if region.region_type == MemoryRegionType::InUse {
                if used_end == region_start {
                    // merge regions
                    if let Some(used_region) = used_region.take() {
                        region.start_addr = used_region.start_addr;
                        region.len += used_region.len;
                    }
                } else if region_end == used_start {
                    // merge regions
                    if let Some(used_region) = used_region.take() {
                        region.len += used_region.len;
                    }
                }
            }
        }

        if let Some(new_region) = new_region {
            self.0.memory_map.push(new_region);
        }
        if let Some(used_region) = used_region {
            self.0.memory_map.push(used_region);
        }
    }
}
