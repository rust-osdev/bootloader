#![no_std]
#![no_main]
#![warn(unsafe_op_in_unsafe_fn)]

use core::{arch::global_asm, slice};
use error::NO_SECOND_STAGE_PARTITION;
use fail::{fail, print_char, UnwrapOrFail};

global_asm!(include_str!("boot.s"));

mod dap;
mod error;
mod fail;
mod mbr;

extern "C" {
    static _mbr_start: u8;
    static _partition_table: u8;
    static _second_stage_start: u8;
}

unsafe fn mbr_start() -> *const u8 {
    unsafe { &_mbr_start }
}

unsafe fn partition_table_raw() -> *const u8 {
    unsafe { &_partition_table }
}

fn second_stage_start() -> *const () {
    let ptr: *const u8 = unsafe { &_second_stage_start };
    ptr as *const ()
}

#[no_mangle]
pub extern "C" fn first_stage(disk_number: u16) {
    // read partition table and look for second stage partition
    print_char(b'1');
    let partition_table = &unsafe { slice::from_raw_parts(partition_table_raw(), 16 * 4) };
    let second_stage_partition =
        mbr::boot_partition(partition_table).unwrap_or_fail(NO_SECOND_STAGE_PARTITION);

    // load second stage partition into memory
    print_char(b'2');
    let target_addr = u16::try_from(second_stage_start() as usize).unwrap_or_fail(b'a');
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

    // jump to second stage
    print_char(b'3');
    let second_stage_entry_point: extern "C" fn(
        disk_number: u16,
        partition_table_start: *const u8,
    ) = unsafe { core::mem::transmute(target_addr as *const ()) };
    let mbr_start = unsafe { partition_table_raw() };
    second_stage_entry_point(disk_number, mbr_start);
    for _ in 0..10 {
        print_char(b'R');
    }

    loop {}
}
