#![no_std]
#![no_main]

use core::{
    arch::{asm, global_asm},
    slice,
};

// mod dap;
mod fail;
// mod fat;
// mod mbr;

#[no_mangle]
pub extern "C" fn _start(disk_number: u16) {
    fail::print_char(b'_');
    fail::print_char(b'_');
    fail::print_char(b'S');
    fail::print_char(b':');

    fail::print_char(b'1');
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

// /// Taken from https://github.com/rust-lang/rust/blob/e100ec5bc7cd768ec17d75448b29c9ab4a39272b/library/core/src/slice/mod.rs#L1673-L1677
// ///
// /// TODO replace with `split_array` feature in stdlib as soon as it's stabilized,
// /// see https://github.com/rust-lang/rust/issues/90091
// fn split_array_ref<const N: usize, T>(slice: &[T]) -> (&[T; N], &[T]) {
//     if N > slice.len() {
//         fail(b'S');
//     }
//     let (a, b) = slice.split_at(N);
//     // SAFETY: a points to [T; N]? Yes it's [T] of length N (checked by split_at)
//     unsafe { (&*(a.as_ptr() as *const [T; N]), b) }
// }
