#[repr(packed)]
#[allow(dead_code)]
pub struct DiskAddressPacket {
    /// Size of the DAP structure
    packet_size: u8,
    /// always zero
    zero: u8,
    /// Number of sectors to transfer
    number_of_sectors: u16,
    /// Offset to memory buffer
    offset: u16,
    /// Segment of memory buffer
    segment: u16,
    /// Start logical block address
    start_lba: u64,
}

impl DiskAddressPacket {
    #[inline(always)]
    pub fn new(memory_buffer_start: u16, file_offset: u64, bytes: u32) -> Self {
        Self {
            packet_size: 0x10,
            zero: 0,
            number_of_sectors: (bytes / 512) as u16,
            offset: memory_buffer_start,
            segment: 0,
            start_lba: file_offset / 512,
        }
    }

    #[inline(always)]
    pub unsafe fn perform_load(&self, disk_number: u16) {
        let self_addr = self as *const Self as u16;
        asm!("
            int 0x13
            jc dap_load_failed",
            in("si") self_addr, in("ax") 0x4200, in("dx") disk_number, out("bx") _,
            options(nostack)
        );
    }
}
