use core::{arch::asm, mem::size_of};

pub static LONG_MODE_GDT: GdtLongMode = GdtLongMode::new();

#[repr(C)]
pub struct GdtLongMode {
    zero: u64,
    code: u64,
    data: u64,
}

impl GdtLongMode {
    const fn new() -> Self {
        let common_flags = {
            (1 << 44) // user segment
            | (1 << 47) // present
            | (1 << 41) // writable
            | (1 << 40) // accessed (to avoid changes by the CPU)
        };
        Self {
            zero: 0,
            code: common_flags | (1 << 43) | (1 << 53), // executable and long mode
            data: common_flags,
        }
    }

    pub fn load(&'static self) {
        let pointer = GdtPointer {
            base: self,
            limit: (3 * size_of::<u64>() - 1) as u16,
        };

        unsafe {
            asm!("lgdt [{}]", in(reg) &pointer, options(readonly, nostack, preserves_flags));
        }
    }
}

#[repr(C, packed(2))]
pub struct GdtPointer {
    /// Size of the DT.
    pub limit: u16,
    /// Pointer to the memory region containing the DT.
    pub base: *const GdtLongMode,
}

unsafe impl Send for GdtPointer {}
unsafe impl Sync for GdtPointer {}
