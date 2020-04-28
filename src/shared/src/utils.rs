#[inline(always)]
pub fn hlt() {
    unsafe {
        llvm_asm!("hlt" :::: "intel","volatile");
    }
}