// Based on https://docs.rs/mbr-nostd

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
            panic!();
        } else if buffer[BUFFER_SIZE - SUFFIX_BYTES.len()..BUFFER_SIZE] != SUFFIX_BYTES[..] {
            panic!();
        }
        let mut entries = [PartitionTableEntry::empty(); MAX_ENTRIES];
        for idx in 0..MAX_ENTRIES {
            let offset = TABLE_OFFSET + idx * ENTRY_SIZE;
            let partition_type = PartitionType::from_mbr_tag_byte(buffer[offset + 4]);
            if let PartitionType::Unknown(c) = partition_type {
                panic!();
            }
            let lba =
                u32::from_le_bytes(buffer[offset + 8..].try_into().unwrap_or_else(|_| panic!()));
            let len = u32::from_le_bytes(
                buffer[offset + 12..]
                    .try_into()
                    .unwrap_or_else(|_| panic!()),
            );
            entries[idx] = PartitionTableEntry::new(partition_type, lba, len);
        }
        MasterBootRecord { entries }
    }

    pub fn partition_table_entries(&self) -> &[PartitionTableEntry] {
        &self.entries[..]
    }
}

/// The type of a particular partition.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
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
}

/// An entry in a partition table.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct PartitionTableEntry {
    /// The type of partition in this entry.
    pub partition_type: PartitionType,

    /// The index of the first block of this entry.
    pub logical_block_address: u32,

    /// The total number of blocks in this entry.
    pub sector_count: u32,
}

impl PartitionTableEntry {
    pub fn new(
        partition_type: PartitionType,
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
        PartitionTableEntry::new(PartitionType::Unused, 0, 0)
    }
}
