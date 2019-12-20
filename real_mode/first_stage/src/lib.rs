#![feature(asm, global_asm)]
#![no_std]

global_asm!(include_str!("boot.s"));

mod dap;

extern "C" {
    fn second_stage(disk_number: u16);
}

/// Test function
#[no_mangle]
pub extern "C" fn first_stage(second_stage_start: u32, second_stage_end: u32, bootloader_start: u32, disk_number: u16) {
    load_second_stage(second_stage_start, second_stage_end, bootloader_start, disk_number);
    unsafe { second_stage(disk_number); }
}

fn load_second_stage(second_stage_start: u32, second_stage_end: u32, bootloader_start: u32, disk_number: u16) {
    use dap::DiskAddressPacket;

    let file_offset = (second_stage_start - bootloader_start) as u64;
    let size = (second_stage_end - second_stage_start) as u32;

    let dap = DiskAddressPacket::new(second_stage_start as u16, file_offset, size);
    unsafe { dap.perform_load(disk_number) }
}

#[no_mangle]
pub extern fn print_char(c: u8) {
    let ax = u16::from(c) | 0x0e00;
    unsafe {
        asm!("int 0x10" :: "{ax}"(ax), "{bx}"(0) :: "intel" );
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
        asm!("hlt":::: "intel","volatile");
    }
}

#[panic_handler]
pub fn panic(_info: &core::panic::PanicInfo) -> ! {
    err(b'P');
}

