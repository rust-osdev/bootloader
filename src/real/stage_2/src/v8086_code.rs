//! V86 code (has to compiled for real mode)
pub fn println(s: &[u8]) {
    print(s);
    print_char(b'\n');
}

pub fn print(s: &[u8]) {
    let mut i = 0;

    while i < s.len() {
        print_char(s[i]);
        i += 1;
    }
}

#[inline(always)]
pub fn print_char(c: u8) {
    let ax = u16::from(c) | 0x0e00;
    unsafe {
        asm!("int 0x10",
            in("ax") ax,
            options(nostack)
        );
    }
}

#[no_mangle]
pub extern "C" fn v8086_test() {
    unsafe { asm!("mov bx, 0xcafe"); }
    unsafe {
    //	asm!("int 0x10", in("ax") 0x41 | 0x0e00, options(nostack));
    }
    loop {};
}