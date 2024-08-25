use super::fail::UnwrapOrFail;

pub(crate) fn get_partition(partitions_raw: &[u8], index: usize) -> PartitionTableEntry {
    const ENTRY_SIZE: usize = 16;

    let offset = index * ENTRY_SIZE;
    let buffer = partitions_raw.get(offset..).unwrap_or_fail(b'c');

    let bootable_raw = *buffer.first().unwrap_or_fail(b'd');
    let bootable = bootable_raw == 0x80;

    let partition_type = *buffer.get(4).unwrap_or_fail(b'e');

    let lba = u32::from_le_bytes(
        buffer
            .get(8..)
            .and_then(|s| s.get(..4))
            .and_then(|s| s.try_into().ok())
            .unwrap_or_fail(b'f'),
    );
    let len = u32::from_le_bytes(
        buffer
            .get(12..)
            .and_then(|s| s.get(..4))
            .and_then(|s| s.try_into().ok())
            .unwrap_or_fail(b'g'),
    );
    PartitionTableEntry::new(bootable, partition_type, lba, len)
}

/// An entry in a partition table.
///
/// Based on https://docs.rs/mbr-nostd
#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) struct PartitionTableEntry {
    /// Whether this partition is a boot partition.
    pub(crate) bootable: bool,

    /// The type of partition in this entry.
    pub(crate) partition_type: u8,

    /// The index of the first block of this entry.
    pub(crate) logical_block_address: u32,

    /// The total number of blocks in this entry.
    pub(crate) sector_count: u32,
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
