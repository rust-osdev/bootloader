use bootloader_api::info::{MemoryRegion, MemoryRegionKind};
use core::{
    iter::{empty, Empty},
    mem::MaybeUninit,
};
use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};

/// A slice of memory that is used by the bootloader and needs to be reserved
/// in the kernel
#[derive(Clone, Copy, Debug)]
pub struct UsedMemorySlice {
    /// the physical start of the slice
    pub start: u64,
    /// The physical end address (exclusive) of the region.
    pub end: u64,
}

impl UsedMemorySlice {
    /// Creates a new slice
    pub fn new_from_len(start: u64, len: u64) -> Self {
        Self {
            start,
            end: start + len,
        }
    }
}

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
pub struct LegacyFrameAllocator<I, D, S> {
    original: I,
    memory_map: I,
    current_descriptor: Option<D>,
    next_frame: PhysFrame,
    min_frame: PhysFrame,
    used_slices: S,
}

/// Start address of the first frame that is not part of the lower 1MB of frames
const LOWER_MEMORY_END_PAGE: u64 = 0x10_0000;

impl<I, D> LegacyFrameAllocator<I, D, Empty<UsedMemorySlice>>
where
    I: ExactSizeIterator<Item = D> + Clone,
    I::Item: LegacyMemoryRegion,
{
    /// Creates a new frame allocator based on the given legacy memory regions.
    ///
    /// Skips the frame at physical address zero to avoid potential problems. For example
    /// identity-mapping the frame at address zero is not valid in Rust, because Rust's `core`
    /// library assumes that references can never point to virtual address `0`.  
    /// Also skips the lower 1MB of frames, there are use cases that require lower conventional memory access (Such as SMP SIPI).
    pub fn new(memory_map: I) -> Self {
        // skip frame 0 because the rust core library does not see 0 as a valid address
        // Also skip at least the lower 1MB of frames, there are use cases that require lower conventional memory access (Such as SMP SIPI).
        let start_frame = PhysFrame::containing_address(PhysAddr::new(LOWER_MEMORY_END_PAGE));
        Self::new_starting_at(start_frame, memory_map)
    }

    /// Creates a new frame allocator based on the given legacy memory regions. Skips any frames
    /// before the given `frame` or `0x10000`(1MB) whichever is higher, there are use cases that require
    /// lower conventional memory access (Such as SMP SIPI).
    pub fn new_starting_at(frame: PhysFrame, memory_map: I) -> Self {
        Self::new_with_used_slices(frame, memory_map, empty())
    }
}

impl<I, D, S> LegacyFrameAllocator<I, D, S>
where
    I: ExactSizeIterator<Item = D> + Clone,
    I::Item: LegacyMemoryRegion,
    S: Iterator<Item = UsedMemorySlice> + Clone,
{
    pub fn new_with_used_slices(start_frame: PhysFrame, memory_map: I, used_slices: S) -> Self {
        // skip frame 0 because the rust core library does not see 0 as a valid address
        // Also skip at least the lower 1MB of frames, there are use cases that require lower conventional memory access (Such as SMP SIPI).
        let lower_mem_end = PhysFrame::containing_address(PhysAddr::new(LOWER_MEMORY_END_PAGE));
        let frame = core::cmp::max(start_frame, lower_mem_end);
        Self {
            original: memory_map.clone(),
            memory_map,
            current_descriptor: None,
            next_frame: frame,
            min_frame: frame,
            used_slices,
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
        let used_slices = Self::used_regions_iter(
            self.min_frame,
            self.next_frame,
            kernel_slice_start,
            kernel_slice_len,
            ramdisk_slice_start,
            ramdisk_slice_len,
            self.used_slices,
        );

        let mut next_index = 0;
        for descriptor in self.original {
            let kind = match descriptor.kind() {
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

            let end = descriptor.start() + descriptor.len();
            let region = MemoryRegion {
                start: descriptor.start().as_u64(),
                end: end.as_u64(),
                kind,
            };
            if region.kind == MemoryRegionKind::Usable {
                Self::split_and_add_region(region, regions, &mut next_index, used_slices.clone());
            } else {
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

    fn used_regions_iter(
        min_frame: PhysFrame,
        next_free: PhysFrame,
        kernel_slice_start: PhysAddr,
        kernel_slice_len: u64,
        ramdisk_slice_start: Option<PhysAddr>,
        ramdisk_slice_len: u64,
        used_slices: S,
    ) -> impl Iterator<Item = UsedMemorySlice> + Clone {
        [
            UsedMemorySlice {
                start: min_frame.start_address().as_u64(),
                end: next_free.start_address().as_u64(),
            },
            UsedMemorySlice::new_from_len(kernel_slice_start.as_u64(), kernel_slice_len),
        ]
        .into_iter()
        .chain(
            ramdisk_slice_start
                .map(|start| UsedMemorySlice::new_from_len(start.as_u64(), ramdisk_slice_len)),
        )
        .chain(used_slices)
    }

    // TODO unit test
    fn split_and_add_region<'a, U>(
        region: MemoryRegion,
        regions: &mut [MaybeUninit<MemoryRegion>],
        next_index: &mut usize,
        used_slices: U,
    ) where
        U: Iterator<Item = UsedMemorySlice> + Clone,
    {
        assert!(region.kind == MemoryRegionKind::Usable);
        if region.start == region.end {
            // skip zero sized regions
            return;
        }

        for slice in used_slices.clone() {
            let slice_end = slice.start + slice.end;
            if region.end <= slice.start || region.start >= slice_end {
                // region and slice don't overlap
                continue;
            }

            if region.start >= slice.start && region.end <= slice_end {
                // region is completly covered by slice
                let bootloader = MemoryRegion {
                    start: region.start,
                    end: region.end,
                    kind: MemoryRegionKind::Bootloader,
                };
                Self::add_region(bootloader, regions, next_index);
                return;
            }
            if region.start < slice.start && region.end <= slice_end {
                // there is a usable region before the bootloader slice
                let before = MemoryRegion {
                    start: region.start,
                    end: slice.start,
                    kind: MemoryRegionKind::Usable,
                };

                let bootloader = MemoryRegion {
                    start: slice.start,
                    end: region.end,
                    kind: MemoryRegionKind::Bootloader,
                };
                Self::split_and_add_region(before, regions, next_index, used_slices);
                Self::add_region(bootloader, regions, next_index);
                return;
            } else if region.start < slice.start && region.end > slice_end {
                // there is usable region before and after the bootloader slice
                let before = MemoryRegion {
                    start: region.start,
                    end: slice.start,
                    kind: MemoryRegionKind::Usable,
                };
                let bootloader = MemoryRegion {
                    start: slice.start,
                    end: slice_end,
                    kind: MemoryRegionKind::Bootloader,
                };
                let after = MemoryRegion {
                    start: slice_end,
                    end: region.end,
                    kind: MemoryRegionKind::Usable,
                };
                Self::split_and_add_region(before, regions, next_index, used_slices.clone());
                Self::add_region(bootloader, regions, next_index);
                Self::split_and_add_region(after, regions, next_index, used_slices.clone());
                return;
            }
            if region.start >= slice.start && region.end > slice_end {
                // there is a usable region after the bootloader slice
                let bootloader = MemoryRegion {
                    start: region.start,
                    end: slice_end,
                    kind: MemoryRegionKind::Bootloader,
                };
                let after = MemoryRegion {
                    start: slice_end,
                    end: region.end,
                    kind: MemoryRegionKind::Usable,
                };
                Self::add_region(bootloader, regions, next_index);
                Self::split_and_add_region(after, regions, next_index, used_slices);
                return;
            }
        }
        // region is not coverd by any slice
        Self::add_region(region, regions, next_index);
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

unsafe impl<I, D, S> FrameAllocator<Size4KiB> for LegacyFrameAllocator<I, D, S>
where
    I: ExactSizeIterator<Item = D> + Clone,
    I::Item: LegacyMemoryRegion,
    S: Iterator<Item = UsedMemorySlice> + Clone,
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
