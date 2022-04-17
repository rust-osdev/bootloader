#![no_std]
#![no_main]

use core::{
    arch::{asm, global_asm},
    slice,
};
use error::NO_SECOND_STAGE_PARTITION;
use fail::{fail, print_char, UnwrapOrFail};

global_asm!(include_str!("boot.s"));

mod dap;
mod error;
mod fail;
mod fat;
mod mbr;

extern "C" {
    static _mbr_start: u8;
    static _partition_table: u8;
    static _second_stage_start: u8;
}

fn mbr_start() -> *const u8 {
    unsafe { &_mbr_start }
}

unsafe fn partition_table() -> *const u8 {
    unsafe { &_partition_table }
}

fn second_stage_start() -> *const () {
    let ptr: *const u8 = unsafe { &_second_stage_start };
    ptr as *const ()
}

#[no_mangle]
pub extern "C" fn first_stage(disk_number: u16) {
    print_char(b'1');
    let partition_table = &unsafe { slice::from_raw_parts(partition_table(), 16 * 4) };
    let second_stage_partition =
        mbr::boot_partition(partition_table).unwrap_or_fail(NO_SECOND_STAGE_PARTITION);

    print_char(b'2');
    let target_addr = u16::try_from(second_stage_start() as usize).unwrap_or_fail(b'a');

    // load boot partition into buffer
    // TODO: only load headers
    let dap = dap::DiskAddressPacket::from_lba(
        target_addr,
        second_stage_partition.logical_block_address.into(),
        second_stage_partition
            .sector_count
            .try_into()
            .unwrap_or_fail(b'b'),
    );
    unsafe {
        dap.perform_load(disk_number);
    }
    if second_stage_partition.sector_count == 0 {
        fail(b'c');
    }

    print_char(b'3');

    // jump to second stage
    let second_stage_entry_point: extern "C" fn(disk_number: u16) =
        unsafe { core::mem::transmute(target_addr as *const ()) };
    unsafe { second_stage_entry_point(disk_number) }

    print_char(b'R');
    print_char(b'R');
    print_char(b'R');
    print_char(b'R');
    print_char(b'R');
    loop {}

    // try to parse FAT file system
    let fat_slice = unsafe {
        slice::from_raw_parts(
            target_addr as *const u8,
            usize::try_from(second_stage_partition.sector_count).unwrap_or_else(|_| fail(b'a'))
                * 512,
        )
    };

    print_char(b'4');
    let boot_sector = fat::BootSector::deserialize(fat_slice);
    let root_dir = boot_sector.bpb.root_dir_first_cluster;
    boot_sector.bpb.check_root_dir();

    print_char(b'5');

    // TODO: get root dir

    // TODO: get offset of `second_stage` file

    // TODO: get offset of `kernel-x86_64` file

    // TODO: load `second_stage` file into memory

    // TODO: jump to `second_stage`, pass offset of `kernel-x86_64` and disk number as arguments

    loop {}
}

fn load_second_stage(
    second_stage_start: u32,
    second_stage_end: u32,
    bootloader_start: u32,
    disk_number: u16,
) {
    use dap::DiskAddressPacket;

    let file_offset = (second_stage_start - bootloader_start) as u64;
    let size = (second_stage_end - second_stage_start) as u32;

    let dap = DiskAddressPacket::new(second_stage_start as u16, file_offset, size);
    unsafe { dap.perform_load(disk_number) }
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
