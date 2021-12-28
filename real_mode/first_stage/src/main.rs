#![no_std]
#![no_main]

use core::{
    arch::{asm, global_asm},
    slice,
};
use mbr::MasterBootRecord;

global_asm!(include_str!("boot.s"));

mod dap;
mod fat;
mod mbr;

extern "C" {
    static _mbr_start: u8;
}

fn mbr_start() -> *const u8 {
    unsafe { &_mbr_start }
}

#[no_mangle]
pub extern "C" fn first_stage(disk_number: u16) {
    let bytes = &unsafe { slice::from_raw_parts(mbr_start(), 512) };
    let mbr = MasterBootRecord::from_bytes(bytes);

    let partition = mbr
        .partition_table_entries()
        .get(0)
        .unwrap_or_else(|| panic!());

    let partition_buf = u16::try_from(mbr_start() as usize).unwrap_or_else(|_| panic!()) + 512;

    // load first partition into buffer
    // TODO: only load headers
    let dap = dap::DiskAddressPacket::from_lba(
        partition_buf,
        partition.logical_block_address.into(),
        partition.sector_count.try_into().unwrap(),
    );
    unsafe {
        dap.perform_load(disk_number);
    }

    // try to parse FAT file system
    let fat_slice = unsafe {
        slice::from_raw_parts(
            partition_buf as *const u8,
            usize::try_from(partition.sector_count).unwrap_or_else(|_| panic!()) * 512,
        )
    };
    let boot_sector = fat::BootSector::deserialize(fat_slice);

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

#[no_mangle]
pub extern "C" fn print_char(c: u8) {
    let ax = u16::from(c) | 0x0e00;
    unsafe {
        asm!("int 0x10", in("ax") ax, in("bx") 0);
    }
}

#[no_mangle]
pub extern "C" fn dap_load_failed() -> ! {
    err(b'1');
}

#[no_mangle]
pub extern "C" fn no_int13h_extensions() -> ! {
    err(b'2');
}

#[cold]
fn err(code: u8) -> ! {
    for &c in b"Err:" {
        print_char(c);
    }
    print_char(code);
    loop {
        hlt()
    }
}

fn hlt() {
    unsafe {
        asm!("hlt");
    }
}

#[panic_handler]
pub fn panic(_info: &core::panic::PanicInfo) -> ! {
    err(b'P');
}
