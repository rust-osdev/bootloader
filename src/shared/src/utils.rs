#[inline(always)]
pub fn hlt() {
    unsafe {
        asm!("hlt" :::: "intel","volatile");
    }
}