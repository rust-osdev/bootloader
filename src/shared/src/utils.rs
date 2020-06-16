#[inline(always)]
pub fn hlt() {
    unsafe {
        asm!("hlt");
    }
}
