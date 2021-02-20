#[no_mangle]
pub unsafe extern fn memcpy(dest: *mut u8, src: *const u8,
                            n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.offset(i as isize) = *src.offset(i as isize);
        i += 1;
    }
    return dest;
}

#[no_mangle]
pub unsafe extern fn memmove(dest: *mut u8, src: *const u8,
                             n: usize) -> *mut u8 {
    if src < dest as *const u8 { // copy from end
        let mut i = n;
        while i != 0 {
            i -= 1;
            *dest.offset(i as isize) = *src.offset(i as isize);
        }
    } else { // copy from beginning
        let mut i = 0;
        while i < n {
            *dest.offset(i as isize) = *src.offset(i as isize);
            i += 1;
        }
    }
    return dest;
}

#[no_mangle]
pub unsafe extern fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *s.offset(i as isize) = c as u8;
        i += 1;
    }
    return s;
}

#[no_mangle]
pub unsafe extern fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = *s1.offset(i as isize);
        let b = *s2.offset(i as isize);
        if a != b {
            return a as i32 - b as i32
        }
        i += 1;
    }
    return 0;
}