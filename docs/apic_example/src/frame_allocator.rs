use bootloader_api::info::MemoryRegionKind::Usable;
use bootloader_api::info::MemoryRegions;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB};
use x86_64::{PhysAddr, VirtAddr};

pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryRegions,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub fn new(memory_map: &'static MemoryRegions) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }
    pub fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();

        let usable_regions = regions.filter(|region| region.kind == Usable);
        let address_ranges = usable_regions.map(|region| region.start..region.end);
        let frame_addresses = address_ranges.flat_map(|region| region.step_by(4096));

        frame_addresses.map(|address| PhysFrame::containing_address(PhysAddr::new(address)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

pub fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level4_table = active_level4_table(physical_memory_offset);
    unsafe { OffsetPageTable::new(level4_table, physical_memory_offset) }
}

fn active_level4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level4_table_frame, _) = Cr3::read();

    let physical_address = level4_table_frame.start_address();
    let virtual_address = physical_memory_offset + physical_address.as_u64();
    let page_table_pointer: *mut PageTable = virtual_address.as_mut_ptr();

    unsafe { &mut *page_table_pointer }
}
