use core::arch::{asm, global_asm};

#[repr(packed)]
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

    pub fn from_lba(memory_buffer_start: u16, start_lba: u64, number_of_sectors: u16) -> Self {
        Self {
            packet_size: 0x10,
            zero: 0,
            number_of_sectors,
            offset: memory_buffer_start,
            segment: 0,
            start_lba,
        }
    }

    pub unsafe fn perform_load(&self, disk_number: u16) {
        let self_addr = self as *const Self as u16;
        asm!("mov {1:x}, si",
            "mov si, {0:x}",
            "int 0x13",
            "jc dap_load_failed",
            "mov si, {1:x}",
            in(reg) self_addr,
            out(reg) _,
            in("ax") 0x4200u16,
            in("dx") disk_number,
        );
    }
}
