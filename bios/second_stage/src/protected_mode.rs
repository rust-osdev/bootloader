use core::{arch::asm, mem::size_of};

static GDT: GdtProtectedMode = GdtProtectedMode::new();

#[repr(C)]
pub struct GdtProtectedMode {
    zero: u64,
    code: u64,
    data: u64,
}

impl GdtProtectedMode {
    const fn new() -> Self {
        let limit = {
            let limit_low = 0xffff;
            let limit_high = 0xf << 48;
            limit_high | limit_low
        };
        let access_common = {
            let present = 1 << 47;
            let user_segment = 1 << 44;
            let read_write = 1 << 41;
            present | user_segment | read_write
        };
        let protected_mode = 1 << 54;
        let granularity = 1 << 55;
        let base_flags = protected_mode | granularity | access_common | limit;
        let executable = 1 << 43;
        Self {
            zero: 0,
            code: base_flags | executable,
            data: base_flags,
        }
    }

    fn clear_interrupts_and_load(&'static self) {
        let pointer = GdtPointer {
            base: &GDT,
            limit: (3 * size_of::<u64>() - 1) as u16,
        };

        unsafe {
            asm!("cli", "lgdt [{}]", in(reg) &pointer, options(readonly, nostack, preserves_flags));
        }
    }
}

#[repr(C, packed(2))]
pub struct GdtPointer {
    /// Size of the DT.
    pub limit: u16,
    /// Pointer to the memory region containing the DT.
    pub base: *const GdtProtectedMode,
}

unsafe impl Send for GdtPointer {}
unsafe impl Sync for GdtPointer {}

pub fn enter_unreal_mode() {
    let ds: u16;
    unsafe {
        asm!("mov {0:x}, ds", out(reg) ds, options(nomem, nostack, preserves_flags));
    }

    GDT.clear_interrupts_and_load();

    // set protected mode bit
    let mut cr0: u32;
    unsafe {
        asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags));
    }
    let cr0_protected = cr0 | 1;
    write_cr0(cr0_protected);

    unsafe {
        asm!("mov bx, 0x10", "mov ds, bx");
    }

    write_cr0(cr0);

    unsafe {
        asm!("mov ds, {0:x}", in(reg) ds, options(nostack, preserves_flags));
        asm!("sti");

        asm!("mov bx, 0x0f01", "mov eax, 0xb8000", "mov [eax], bx");
    }
}

pub unsafe fn copy_to_protected_mode(target: *mut u8, bytes: &[u8]) {
    for (offset, byte) in bytes.iter().enumerate() {
        let dst = target.wrapping_add(offset);
        unsafe { asm!("mov [{}], {}", in(reg) dst, in(reg) byte) };
    }
}

fn write_cr0(val: u32) {
    unsafe { asm!("mov cr0, {}", in(reg) val, options(nostack, preserves_flags)) };
}
