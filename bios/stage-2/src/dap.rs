use core::arch::asm;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
#[repr(C, packed)]
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
    pub fn from_lba(
        start_lba: u64,
        number_of_sectors: u16,
        target_addr: u16,
        target_addr_segment: u16,
    ) -> Self {
        Self {
            packet_size: 0x10,
            zero: 0,
            number_of_sectors,
            offset: target_addr,
            segment: target_addr_segment,
            start_lba,
        }
    }

    pub unsafe fn perform_load(&self, disk_number: u16) {
        let self_addr = self as *const Self as u16;
        asm!(
            "push 'z'", // error code `z`, passed to `fail` on error
            "mov {1:x}, si",
            "mov si, {0:x}",
            "int 0x13",
            "jnc 2f", // carry is set on fail
            "call fail",
            "2:",
            "pop si", // remove error code again
            "mov si, {1:x}",
            in(reg) self_addr,
            out(reg) _,
            in("ax") 0x4200u16,
            in("dx") disk_number,
        );
    }
}
