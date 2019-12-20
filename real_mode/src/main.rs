#![feature(asm, global_asm)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod dap;

global_asm!(include_str!("boot.s"));
global_asm!(include_str!("second_stage.s"));

extern "C" {
    fn second_stage() -> u32;
}

#[allow(improper_ctypes)]
extern "C" {
    static _bootloader_start: ();
    static _second_stage_start: ();
    static _second_stage_end: ();
}

#[no_mangle]
pub extern "C" fn rust_main(disk_number: u16) {
    load_second_stage(disk_number);

    let val = unsafe { second_stage() };
    if val == 12345 {
        println(b"match");
    } else {
        println(b"no match");
    }
}

fn bootloader_start() -> usize {
    unsafe { &_bootloader_start as *const _ as usize }
}

fn second_stage_start() -> usize {
    unsafe { &_second_stage_start as *const _ as usize }
}

fn second_stage_end() -> usize {
    unsafe { &_second_stage_end as *const _ as usize }
}

fn load_second_stage(disk_number: u16) {
    use dap::DiskAddressPacket;

    let file_offset = (second_stage_start() - bootloader_start()) as u64;
    let size = (second_stage_end() - second_stage_start()) as u32;

    let dap = DiskAddressPacket::new(second_stage_start() as u16, file_offset, size);
    unsafe { dap.perform_load(disk_number) }
}

#[inline(never)]
fn println(s: &[u8]) {
    print(s);
    print_char(b'\n');
}

fn print(s: &[u8]) {
    for &c in s {
        print_char(c);
    }
}

fn print_char(c: u8) {
    let ax = u16::from(c) | 0x0e00;
    unsafe {
        asm!("int 0x10" :: "{ax}"(ax) :: "intel" );
    }
}

#[no_mangle]
pub extern "C" fn dap_load_failed() -> ! {
    println(b"ERROR: DAP load failed");
    loop {
        hlt()
    }
}

#[no_mangle]
pub extern "C" fn no_int13h_extensions() -> ! {
    println(b"ERROR: No int13h extensions");
    loop {
        hlt()
    }
}

#[panic_handler]
pub fn panic(_info: &PanicInfo) -> ! {
    println(b"PANIC!");
    loop {
        hlt()
    }
}

fn hlt() {
    unsafe {
        asm!("hlt":::: "intel","volatile");
    }
}
