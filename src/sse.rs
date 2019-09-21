/// Enables Streaming SIMD Extensions (SSE) support for loaded kernels.
pub fn enable_sse() {
    use bit_field::BitField;
    use x86_64::registers::control::Cr0;
    let mut flags = Cr0::read_raw();
    flags.set_bit(2, false);
    flags.set_bit(1, true);
    flags.set_bit(9, true);
    flags.set_bit(10, true);
    unsafe {
        Cr0::write_raw(flags);
    }
    // For now, we must use inline ASM here
    let mut cr4: u64;
    unsafe {
        asm!("mov %cr4, $0" : "=r" (cr4));
    }
    cr4.set_bit(9, true);
    cr4.set_bit(10, true);
    unsafe {
        asm!("mov $0, %cr4" :: "r" (cr4) : "memory");
    }
}
