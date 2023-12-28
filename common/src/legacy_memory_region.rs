use bootloader_api::info::{MemoryRegion, MemoryRegionKind};
use core::mem::MaybeUninit;
use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};

/// Abstraction trait for a memory region returned by the UEFI or BIOS firmware.
pub trait LegacyMemoryRegion: Copy + core::fmt::Debug {
    /// Returns the physical start address of the region.
    fn start(&self) -> PhysAddr;
    /// Returns the size of the region in bytes.
    fn len(&self) -> u64;
    /// Returns whether this region is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Returns the type of the region, e.g. whether it is usable or reserved.
    fn kind(&self) -> MemoryRegionKind;

    /// Some regions become usable when the bootloader jumps to the kernel.
    fn usable_after_bootloader_exit(&self) -> bool;
}

/// A physical frame allocator based on a BIOS or UEFI provided memory map.
pub struct LegacyFrameAllocator<I, D> {
    original: I,
    memory_map: I,
    current_descriptor: Option<D>,
    next_frame: PhysFrame,
}

impl<I, D> LegacyFrameAllocator<I, D>
where
    I: ExactSizeIterator<Item = D> + Clone,
    I::Item: LegacyMemoryRegion,
{
    /// Creates a new frame allocator based on the given legacy memory regions.
    ///
    /// Skips the frame at physical address zero to avoid potential problems. For example
    /// identity-mapping the frame at address zero is not valid in Rust, because Rust's `core`
    /// library assumes that references can never point to virtual address `0`.  
    pub fn new(memory_map: I) -> Self {
        // skip frame 0 because the rust core library does not see 0 as a valid address
        let start_frame = PhysFrame::containing_address(PhysAddr::new(0x1000));
        Self::new_starting_at(start_frame, memory_map)
    }

    /// Creates a new frame allocator based on the given legacy memory regions. Skips any frames
    /// before the given `frame`.
    pub fn new_starting_at(frame: PhysFrame, memory_map: I) -> Self {
        Self {
            original: memory_map.clone(),
            memory_map,
            current_descriptor: None,
            next_frame: frame,
        }
    }

    fn allocate_frame_from_descriptor(&mut self, descriptor: D) -> Option<PhysFrame> {
        let start_addr = descriptor.start();
        let start_frame = PhysFrame::containing_address(start_addr);
        let end_addr = start_addr + descriptor.len();
        let end_frame = PhysFrame::containing_address(end_addr - 1u64);

        // increase self.next_frame to start_frame if smaller
        if self.next_frame < start_frame {
            self.next_frame = start_frame;
        }

        if self.next_frame <= end_frame {
            let ret = self.next_frame;
            self.next_frame += 1;
            Some(ret)
        } else {
            None
        }
    }

    /// Returns the number of memory regions in the underlying memory map.
    ///
    /// The function always returns the same value, i.e. the length doesn't
    /// change after calls to `allocate_frame`.
    pub fn len(&self) -> usize {
        self.original.len()
    }

    /// Returns whether this memory map is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the largest detected physical memory address.
    ///
    /// Useful for creating a mapping for all physical memory.
    pub fn max_phys_addr(&self) -> PhysAddr {
        self.original
            .clone()
            .map(|r| r.start() + r.len())
            .max()
            .unwrap()
    }

    /// Converts this type to a boot info memory map.
    ///
    /// The memory map is placed in the given `regions` slice. The length of the given slice
    /// must be at least the value returned by [`len`] plus 1.
    ///
    /// The return slice is a subslice of `regions`, shortened to the actual number of regions.
    pub fn construct_memory_map(
        self,
        regions: &mut [MaybeUninit<MemoryRegion>],
        kernel_slice_start: PhysAddr,
        kernel_slice_len: u64,
        ramdisk_slice_start: Option<PhysAddr>,
        ramdisk_slice_len: u64,
    ) -> &mut [MemoryRegion] {
        let mut next_index = 0;
        let kernel_slice_start = kernel_slice_start.as_u64();
        let ramdisk_slice_start = ramdisk_slice_start.map(|a| a.as_u64());

        for descriptor in self.original {
            let mut start = descriptor.start();
            let end = start + descriptor.len();
            let next_free = self.next_frame.start_address();
            let kind = match descriptor.kind() {
                MemoryRegionKind::Usable => {
                    if end <= next_free {
                        MemoryRegionKind::Bootloader
                    } else if descriptor.start() >= next_free {
                        MemoryRegionKind::Usable
                    } else {
                        // part of the region is used -> add it separately
                        let used_region = MemoryRegion {
                            start: descriptor.start().as_u64(),
                            end: next_free.as_u64(),
                            kind: MemoryRegionKind::Bootloader,
                        };
                        Self::add_region(used_region, regions, &mut next_index);

                        // add unused part normally
                        start = next_free;
                        MemoryRegionKind::Usable
                    }
                }
                _ if descriptor.usable_after_bootloader_exit() => {
                    // Region was not usable before, but it will be as soon as
                    // the bootloader passes control to the kernel. We don't
                    // need to check against `next_free` because the
                    // LegacyFrameAllocator only allocates memory from usable
                    // descriptors.
                    MemoryRegionKind::Usable
                }
                other => other,
            };

            let region = MemoryRegion {
                start: start.as_u64(),
                end: end.as_u64(),
                kind,
            };

            // check if region overlaps with kernel or ramdisk
            let kernel_slice_end = kernel_slice_start + kernel_slice_len;
            let ramdisk_slice_end = ramdisk_slice_start.map(|s| s + ramdisk_slice_len);
            if region.kind == MemoryRegionKind::Usable
                && kernel_slice_start < region.end
                && kernel_slice_end > region.start
            {
                // region overlaps with kernel -> we might need to split it

                // ensure that the kernel allocation does not span multiple regions
                assert!(
                    kernel_slice_start >= region.start,
                    "region overlaps with kernel, but kernel begins before region \
                    (kernel_slice_start: {kernel_slice_start:#x}, region_start: {:#x})",
                    region.start
                );
                assert!(
                    kernel_slice_end <= region.end,
                    "region overlaps with kernel, but region ends before kernel \
                    (kernel_slice_end: {kernel_slice_end:#x}, region_end: {:#x})",
                    region.end,
                );

                // split the region into three parts
                let before_kernel = MemoryRegion {
                    end: kernel_slice_start,
                    ..region
                };
                let kernel = MemoryRegion {
                    start: kernel_slice_start,
                    end: kernel_slice_end,
                    kind: MemoryRegionKind::Bootloader,
                };
                let after_kernel = MemoryRegion {
                    start: kernel_slice_end,
                    ..region
                };

                // add the three regions (empty regions are ignored in `add_region`)
                Self::add_region(before_kernel, regions, &mut next_index);
                Self::add_region(kernel, regions, &mut next_index);
                Self::add_region(after_kernel, regions, &mut next_index);
            } else if region.kind == MemoryRegionKind::Usable
                && ramdisk_slice_start.map(|s| s < region.end).unwrap_or(false)
                && ramdisk_slice_end.map(|e| e > region.start).unwrap_or(false)
            {
                // region overlaps with ramdisk -> we might need to split it
                let ramdisk_slice_start = ramdisk_slice_start.unwrap();
                let ramdisk_slice_end = ramdisk_slice_end.unwrap();

                // ensure that the ramdisk allocation does not span multiple regions
                assert!(
                    ramdisk_slice_start >= region.start,
                    "region overlaps with ramdisk, but ramdisk begins before region \
                (ramdisk_start: {ramdisk_slice_start:#x}, region_start: {:#x})",
                    region.start
                );
                assert!(
                    ramdisk_slice_end <= region.end,
                    "region overlaps with ramdisk, but region ends before ramdisk \
                (ramdisk_end: {ramdisk_slice_end:#x}, region_end: {:#x})",
                    region.end,
                );

                // split the region into three parts
                let before_ramdisk = MemoryRegion {
                    end: ramdisk_slice_start,
                    ..region
                };
                let ramdisk = MemoryRegion {
                    start: ramdisk_slice_start,
                    end: ramdisk_slice_end,
                    kind: MemoryRegionKind::Bootloader,
                };
                let after_ramdisk = MemoryRegion {
                    start: ramdisk_slice_end,
                    ..region
                };

                // add the three regions (empty regions are ignored in `add_region`)
                Self::add_region(before_ramdisk, regions, &mut next_index);
                Self::add_region(ramdisk, regions, &mut next_index);
                Self::add_region(after_ramdisk, regions, &mut next_index);
            } else {
                // add the region normally
                Self::add_region(region, regions, &mut next_index);
            }
        }

        let initialized = &mut regions[..next_index];
        unsafe {
            // inlined variant of: `MaybeUninit::slice_assume_init_mut(initialized)`
            // TODO: undo inlining when `slice_assume_init_mut` becomes stable
            &mut *(initialized as *mut [_] as *mut [_])
        }
    }

    fn add_region(
        region: MemoryRegion,
        regions: &mut [MaybeUninit<MemoryRegion>],
        next_index: &mut usize,
    ) {
        if region.start == region.end {
            // skip zero sized regions
            return;
        }
        unsafe {
            regions
                .get_mut(*next_index)
                .expect("cannot add region: no more free entries in memory map")
                .as_mut_ptr()
                .write(region)
        };
        *next_index += 1;
    }
}

unsafe impl<I, D> FrameAllocator<Size4KiB> for LegacyFrameAllocator<I, D>
where
    I: ExactSizeIterator<Item = D> + Clone,
    I::Item: LegacyMemoryRegion,
{
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        if let Some(current_descriptor) = self.current_descriptor {
            match self.allocate_frame_from_descriptor(current_descriptor) {
                Some(frame) => return Some(frame),
                None => {
                    self.current_descriptor = None;
                }
            }
        }

        // find next suitable descriptor
        while let Some(descriptor) = self.memory_map.next() {
            if descriptor.kind() != MemoryRegionKind::Usable {
                continue;
            }
            if let Some(frame) = self.allocate_frame_from_descriptor(descriptor) {
                self.current_descriptor = Some(descriptor);
                return Some(frame);
            }
        }

        None
    }
}
