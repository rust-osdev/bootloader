// Based on https://docs.rs/mbr-nostd

use super::fail::{fail, UnwrapOrFail};

/// Returns the first bootable partition in the partition table.
pub fn boot_partition(partitions_raw: &[u8]) -> Option<PartitionTableEntry> {
    for index in 0..4 {
        let entry = get_partition(partitions_raw, index);
        if entry.bootable {
            return Some(entry);
        }
    }
    None
}

pub fn get_partition(partitions_raw: &[u8], index: usize) -> PartitionTableEntry {
    if partitions_raw.len() < PARTITIONS_AREA_SIZE {
        fail(b'a');
    }

    let offset = index * ENTRY_SIZE;
    let buffer = partitions_raw.get(offset..).unwrap_or_fail(b'c');

    let bootable_raw = *buffer.get(0).unwrap_or_fail(b'd');
    let bootable = bootable_raw == 0x80;

    let partition_type = *buffer.get(4).unwrap_or_fail(b'e');

    let lba = u32::from_le_bytes(
        buffer
            .get(8..)
            .and_then(|s| s.get(..4))
            .and_then(|s| s.try_into().ok())
            .unwrap_or_fail(b'e'),
    );
    let len = u32::from_le_bytes(
        buffer
            .get(12..)
            .and_then(|s| s.get(..4))
            .and_then(|s| s.try_into().ok())
            .unwrap_or_fail(b'f'),
    );
    PartitionTableEntry::new(bootable, partition_type, lba, len)
}

const PARTITIONS_AREA_SIZE: usize = 16 * 4;
const ENTRY_SIZE: usize = 16;

/// The type of a particular partition.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum PartitionType {
    Unused,
    Unknown(u8),
    Fat12(u8),
    Fat16(u8),
    Fat32(u8),
    LinuxExt(u8),
    HfsPlus(u8),
    ISO9660(u8),
    NtfsExfat(u8),
}

impl PartitionType {
    /// Parses a partition type from the type byte in the MBR's table.
    pub fn from_mbr_tag_byte(tag: u8) -> PartitionType {
        match tag {
            0x0 => PartitionType::Unused,
            0x01 => PartitionType::Fat12(tag),
            0x04 | 0x06 | 0x0e => PartitionType::Fat16(tag),
            0x0b | 0x0c | 0x1b | 0x1c => PartitionType::Fat32(tag),
            0x83 => PartitionType::LinuxExt(tag),
            0x07 => PartitionType::NtfsExfat(tag),
            0xaf => PartitionType::HfsPlus(tag),
            _ => PartitionType::Unknown(tag),
        }
    }

    pub fn known_type(tag: u8) -> bool {
        match tag {
            0x0 | 0x01 | 0x04 | 0x06 | 0x0e | 0x0b | 0x0c | 0x1b | 0x1c | 0x83 | 0x07 | 0xaf => {
                true
            }
            _ => false,
        }
    }
}

/// An entry in a partition table.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct PartitionTableEntry {
    /// Whether this partition is a boot partition.
    pub bootable: bool,

    /// The type of partition in this entry.
    pub partition_type: u8,

    /// The index of the first block of this entry.
    pub logical_block_address: u32,

    /// The total number of blocks in this entry.
    pub sector_count: u32,
}

impl PartitionTableEntry {
    pub fn new(
        bootable: bool,
        partition_type: u8,
        logical_block_address: u32,
        sector_count: u32,
    ) -> PartitionTableEntry {
        PartitionTableEntry {
            bootable,
            partition_type,
            logical_block_address,
            sector_count,
        }
    }
}
