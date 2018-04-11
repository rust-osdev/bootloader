use os_bootinfo::{MemoryMap, MemoryRegion, MemoryRegionType};
use x86_64::structures::paging::{PhysFrame, PageSize, PhysFrameRange};
use x86_64::PhysAddr;

pub(crate) struct FrameAllocator<'a> {
    pub memory_map: &'a mut MemoryMap,
}

impl<'a> FrameAllocator<'a> {
    pub(crate) fn allocate_frame(&mut self, region_type: MemoryRegionType) -> Option<PhysFrame> {
        // try to find an existing region of same type that can be enlarged
        let mut iter = self.memory_map.iter_mut().peekable();
        while let Some(region) = iter.next() {
            if region.region_type == region_type {
                if let Some(next) = iter.peek() {
                    if next.range.start == region.range.end && next.region_type == MemoryRegionType::Usable && !next.range.is_empty() {
                        let frame = region.range.end;
                        region.range.end += 1;
                        iter.next().unwrap().range.start += 1;
                        return Some(frame);
                    }
                }
            }
        }

        fn split_usable_region<'a, I>(iter: &mut I) -> Option<(PhysFrame, PhysFrameRange)>
        where
            I: Iterator<Item = &'a mut MemoryRegion>,
        {
            for region in iter {
                if region.region_type != MemoryRegionType::Usable {
                    continue;
                }
                if region.range.is_empty() {
                    continue;
                }

                let frame = region.range.start;
                region.range.start += 1;
                return Some((frame, PhysFrame::range(frame, frame + 1)));
            }
            None
        }

        let result = if region_type == MemoryRegionType::PageTable {
            // prevent fragmentation when page tables are allocated in between
            split_usable_region(&mut self.memory_map.iter_mut().rev())
        } else {
            split_usable_region(&mut self.memory_map.iter_mut())
        };

        if let Some((frame, range)) = result {
            self.memory_map.add_region(MemoryRegion { range, region_type, });
            Some(frame)
        } else {
            None
        }
    }

    pub(crate) fn deallocate_frame(&mut self, frame: PhysFrame) {
        // try to find an existing region of same type that can be enlarged
        let mut iter = self.memory_map.iter_mut().peekable();
        while let Some(region) = iter.next() {
            if region.range.end == frame && region.region_type == MemoryRegionType::Usable {
                region.range.end += 1;
                if let Some(next) = iter.next() {
                    if next.range.start == frame {
                        next.range.start += 1;
                    }
                }
                return;
            }
        }

        // insert frame as a new region
        self.memory_map.add_region(MemoryRegion {
            range: PhysFrame::range(frame, frame + 1),
            region_type: MemoryRegionType::Usable,
        });
    }

    /// Marks the passed region in the memory map.
    ///
    /// Panics if a non-usable region (e.g. a reserved region) overlaps with the passed region.
    pub(crate) fn mark_allocated_region(&mut self, region: MemoryRegion) {
        for r in self.memory_map.iter_mut() {
            if region.range.start >= r.range.end {
                continue
            }
            if region.range.end <= r.range.start {
                continue
            }

            if r.region_type != MemoryRegionType::Usable {
                panic!("region {:x?} overlaps with non-usable region {:x?}", region, r);
            }

            if region.range.start == r.range.start {
                if region.range.end < r.range.end {
                    // Case: (r = `r`, R = `region`)
                    // ----rrrrrrrrrrr----
                    // ----RRRR-----------
                    r.range.start = region.range.end;
                    self.memory_map.add_region(region);
                } else {
                    // Case: (r = `r`, R = `region`)
                    // ----rrrrrrrrrrr----
                    // ----RRRRRRRRRRRRRR-
                    *r = region;
                }
            } else if region.range.start > r.range.start {
                if region.range.end < r.range.end {
                    // Case: (r = `r`, R = `region`)
                    // ----rrrrrrrrrrr----
                    // ------RRRR---------
                    let mut behind_r = r.clone();
                    behind_r.range.start = region.range.end;
                    r.range.end = region.range.start;
                    self.memory_map.add_region(behind_r);
                    self.memory_map.add_region(region);
                } else {
                    // Case: (r = `r`, R = `region`)
                    // ----rrrrrrrrrrr----
                    // -----------RRRR---- or
                    // -------------RRRR--
                    r.range.end = region.range.start;
                    self.memory_map.add_region(region);
                }
            } else {
                // Case: (r = `r`, R = `region`)
                // ----rrrrrrrrrrr----
                // --RRRR-------------
                r.range.start = region.range.end;
                self.memory_map.add_region(region);
            }
            return;
        }
        panic!("region {:x?} is not a usable memory region", region);
    }
}
