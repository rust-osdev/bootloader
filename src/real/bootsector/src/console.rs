#[no_mangle]
pub fn real_mode_println(s: &[u8]) {
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
        llvm_asm!("int 0x10" :: "{ax}"(ax) :: "intel" );
    }
}
