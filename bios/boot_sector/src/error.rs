/// The boot sector did not find the second stage partition.
///
/// The BIOS bootloader requires a special second stage partition with partition type 0x20.
pub const NO_SECOND_STAGE_PARTITION: u8 = b'x';
