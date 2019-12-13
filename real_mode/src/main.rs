#![feature(asm, global_asm)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;

global_asm!(include_str!("boot.s"));

#[no_mangle]
pub extern "C" fn rust_main() {
    println(b"Hello from Rust!");
    panic!()
}

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

#[panic_handler]
pub fn panic(_info: &PanicInfo) -> ! {
    println(b"PANIC!");
    loop {}
}

