/// Enables Streaming SIMD Extensions (SSE) support for loaded kernels.
pub fn enable_sse() {
    use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags};
    let mut flags = Cr0::read();
    flags.remove(Cr0Flags::EMULATE_COPROCESSOR);
    flags.insert(Cr0Flags::MONITOR_COPROCESSOR);
    unsafe {
        Cr0::write(flags);
    }

    let mut flags = Cr4::read();
    flags.insert(Cr4Flags::OSFXSR);
    flags.insert(Cr4Flags::OSXMMEXCPT_ENABLE);
    unsafe {
        Cr4::write(flags);
    }
}
