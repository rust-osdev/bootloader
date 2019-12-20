#![feature(asm, global_asm)]
#![no_std]

global_asm!(include_str!("second_stage.s"));

extern "C" {
    fn print_char(c: u8);
    fn second_stage_asm() -> u32;
}

#[no_mangle]
pub extern "C" fn second_stage(_disk_number: u16) {
    let val = unsafe { second_stage_asm() };
    if val == 12345 {
        println(b"match");
    } else {
        println(b"no match");
    }
}

#[panic_handler]
pub fn panic(_info: &core::panic::PanicInfo) -> ! {
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

#[inline(never)]
fn println(s: &[u8]) {
    print(s);
    unsafe { print_char(b'\n') };
}

fn print(s: &[u8]) {
    for &c in s {
        unsafe { print_char(c) };
    }
}
