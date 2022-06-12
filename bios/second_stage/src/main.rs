#![no_std]
#![no_main]

use byteorder::{ByteOrder, LittleEndian};
use core::{fmt::Write as _, slice};
use disk::Read;
use mbr_nostd::{PartitionTableEntry, PartitionType};

mod dap;
mod disk;
mod fat;
// mod fat_old;
// mod fat_bpb;
// mod mini_fat;
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
    write!(screen::Writer, "\nSECOND STAGE: ").unwrap();

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
    screen::print_char(b'1');

    // load fat partition
    let mut disk = disk::DiskAccess {
        disk_number,
        base_offset: u64::from(fat_partition.logical_block_address) * 512,
        current_offset: 0,
    };

    let mut fs = fat::FileSystem::parse(disk.clone());
    let kernel = fs
        .lookup_file("kernel-x86_64")
        .expect("no `kernel-x86_64` file found");
    screen::print_char(b'2');

    let mut buffer = [0u8; 512];
    disk.read_exact(&mut buffer);
    screen::print_char(b'3');

    let kernel_first_cluster = todo!();
    loop {}

    // try to parse FAT file system
    // let fat_slice = unsafe {
    //     slice::from_raw_parts(
    //         partition_buf as *const u8,
    //         usize::try_from(second_stage_partition.sector_count).unwrap_or_else(|_| fail(b'a'))
    //             * 512,
    //     )
    // };

    // print_char(b'4');
    // let boot_sector = fat::BootSector::deserialize(fat_slice);
    // let root_dir = boot_sector.bpb.root_dir_first_cluster;
    // boot_sector.bpb.check_root_dir();

    // print_char(b'5');

    // TODO: get root dir

    // TODO: get offset of `second_stage` file

    // TODO: get offset of `kernel-x86_64` file

    // TODO: load `second_stage` file into memory

    // TODO: jump to `second_stage`, pass offset of `kernel-x86_64` and disk number as arguments

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
