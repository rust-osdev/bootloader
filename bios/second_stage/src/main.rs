#![no_std]
#![no_main]

use byteorder::{ByteOrder, LittleEndian};
use core::{arch::asm, fmt::Write as _, mem::size_of, slice};
use mbr_nostd::{PartitionTableEntry, PartitionType};

use crate::protected_mode::enter_unreal_mode;

mod dap;
mod disk;
mod fat;
mod protected_mode;
mod screen;

/// We use this partition type to store the second bootloader stage;
const BOOTLOADER_SECOND_STAGE_PARTITION_TYPE: u8 = 0x20;

extern "C" {
    static _second_stage_end: u8;
}

fn second_stage_end() -> *const u8 {
    unsafe { &_second_stage_end }
}

#[no_mangle]
#[link_section = ".start"]
pub extern "C" fn _start(disk_number: u16, partition_table_start: *const u8) {
    screen::Writer.write_str(" -> SECOND STAGE").unwrap();

    enter_unreal_mode();

    // parse partition table
    let partitions = {
        const MAX_ENTRIES: usize = 4;
        const ENTRY_SIZE: usize = 16;

        let mut entries = [PartitionTableEntry::empty(); MAX_ENTRIES];
        let raw = unsafe { slice::from_raw_parts(partition_table_start, ENTRY_SIZE * MAX_ENTRIES) };
        for idx in 0..MAX_ENTRIES {
            let offset = idx * ENTRY_SIZE;
            let partition_type = PartitionType::from_mbr_tag_byte(raw[offset + 4]);
            let lba = LittleEndian::read_u32(&raw[offset + 8..]);
            let len = LittleEndian::read_u32(&raw[offset + 12..]);
            entries[idx] = PartitionTableEntry::new(partition_type, lba, len);
        }
        entries
    };
    // look for second stage partition
    let second_stage_partition_idx = partitions
        .iter()
        .enumerate()
        .find(|(_, e)| {
            e.partition_type == PartitionType::Unknown(BOOTLOADER_SECOND_STAGE_PARTITION_TYPE)
        })
        .unwrap()
        .0;
    let fat_partition = partitions.get(second_stage_partition_idx + 1).unwrap();
    assert!(matches!(
        fat_partition.partition_type,
        PartitionType::Fat12(_) | PartitionType::Fat16(_) | PartitionType::Fat32(_)
    ));

    // load fat partition
    let mut disk = disk::DiskAccess {
        disk_number,
        base_offset: u64::from(fat_partition.logical_block_address) * 512,
        current_offset: 0,
    };

    let mut fs = fat::FileSystem::parse(disk.clone());

    let kernel = fs
        .find_file_in_root_dir("kernel-x86_64")
        .expect("no `kernel-x86_64` file found");

    for cluster in fs.file_clusters(&kernel) {
        let cluster = cluster.unwrap();
        writeln!(
            screen::Writer,
            "kernel cluster: start: {:#x}, len: {}",
            cluster.start_offset,
            cluster.len_bytes
        )
        .unwrap();
    }

    writeln!(screen::Writer, "DONE").unwrap();

    // TODO: Retrieve memory map

    // TODO: Set up protected mode, or unreal mode

    // TODO: Load `kernel` to protected mode address

    // TODO: Set up long mode with identity-mapping

    // TODO: Load third stage that uses `bootloader-common` crate and jump to it

    loop {}
}

/// Taken from https://github.com/rust-lang/rust/blob/e100ec5bc7cd768ec17d75448b29c9ab4a39272b/library/core/src/slice/mod.rs#L1673-L1677
///
/// TODO replace with `split_array` feature in stdlib as soon as it's stabilized,
/// see https://github.com/rust-lang/rust/issues/90091
fn split_array_ref<const N: usize, T>(slice: &[T]) -> (&[T; N], &[T]) {
    if N > slice.len() {
        fail(b'S');
    }
    let (a, b) = slice.split_at(N);
    // SAFETY: a points to [T; N]? Yes it's [T] of length N (checked by split_at)
    unsafe { (&*(a.as_ptr() as *const [T; N]), b) }
}

#[cold]
#[inline(never)]
#[no_mangle]
pub extern "C" fn fail(code: u8) -> ! {
    panic!("fail: {}", code as char);
}
