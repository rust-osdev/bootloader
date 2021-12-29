// Based on https://docs.rs/mbr-nostd

use super::fail::{fail, UnwrapOrFail};

pub fn get_partition(buffer: &[u8], index: usize) -> PartitionTableEntry {
    if buffer.len() < BUFFER_SIZE {
        fail(b'a');
    } else if buffer.get(BUFFER_SIZE - SUFFIX_BYTES.len()..BUFFER_SIZE) != Some(&SUFFIX_BYTES[..]) {
        fail(b'b');
    }

    let offset = TABLE_OFFSET + index * ENTRY_SIZE;
    let buffer = buffer.get(offset..).unwrap_or_fail(b'c');

    let partition_type = *buffer.get(4).unwrap_or_fail(b'd');

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
    PartitionTableEntry::new(partition_type, lba, len)
}

/// A struct representing an MBR partition table.
pub struct MasterBootRecord {
    entries: [PartitionTableEntry; MAX_ENTRIES],
}

const BUFFER_SIZE: usize = 512;
const TABLE_OFFSET: usize = 446;
const ENTRY_SIZE: usize = 16;
const SUFFIX_BYTES: [u8; 2] = [0x55, 0xaa];
const MAX_ENTRIES: usize = (BUFFER_SIZE - TABLE_OFFSET - 2) / ENTRY_SIZE;

impl MasterBootRecord {
    /// Parses the MBR table from a raw byte buffer.

    pub fn from_bytes(buffer: &[u8]) -> MasterBootRecord {
        if buffer.len() < BUFFER_SIZE {
            fail(b'1');
        } else if buffer.get(BUFFER_SIZE - SUFFIX_BYTES.len()..BUFFER_SIZE)
            != Some(&SUFFIX_BYTES[..])
        {
            fail(b'2');
        }
        let mut entries = [PartitionTableEntry::empty(); MAX_ENTRIES];

        for idx in 0..MAX_ENTRIES {
            let offset = TABLE_OFFSET + idx * ENTRY_SIZE;
            let buffer = buffer.get(offset..).unwrap_or_fail(b'8');

            let partition_type = *buffer.get(4).unwrap_or_fail(b'4');

            let lba = u32::from_le_bytes(
                buffer
                    .get(8..)
                    .and_then(|s| s.get(..4))
                    .and_then(|s| s.try_into().ok())
                    .unwrap_or_fail(b'5'),
            );
            let len = u32::from_le_bytes(
                buffer
                    .get(12..)
                    .and_then(|s| s.get(..4))
                    .and_then(|s| s.try_into().ok())
                    .unwrap_or_fail(b'6'),
            );
            *entries.get_mut(idx).unwrap_or_fail(b'7') =
                PartitionTableEntry::new(partition_type, lba, len);
        }

        MasterBootRecord { entries }
    }

    pub fn partition_table_entries(&self) -> &[PartitionTableEntry] {
        &self.entries[..]
    }
}

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
    /// The type of partition in this entry.
    pub partition_type: u8,

    /// The index of the first block of this entry.
    pub logical_block_address: u32,

    /// The total number of blocks in this entry.
    pub sector_count: u32,
}

impl PartitionTableEntry {
    pub fn new(
        partition_type: u8,
        logical_block_address: u32,
        sector_count: u32,
    ) -> PartitionTableEntry {
        PartitionTableEntry {
            partition_type,
            logical_block_address,
            sector_count,
        }
    }

    pub fn empty() -> PartitionTableEntry {
        PartitionTableEntry::new(0, 0, 0)
    }
}
