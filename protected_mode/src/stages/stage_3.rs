#[no_mangle]
extern "C" fn stage_3() -> ! {
    let ptr = 0xb8200 as *mut u16;
    unsafe {
        *ptr = 0xffff;
    }
    loop {}
}
