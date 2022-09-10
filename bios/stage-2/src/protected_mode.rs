use bootloader_x86_64_bios_common::BiosInfo;
use core::{
    arch::{asm, global_asm},
    fmt::Write as _,
    mem::size_of,
};

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
            base: self,
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
    let cr0 = set_protected_mode_bit();

    // load GDT
    unsafe {
        asm!("mov bx, 0x10", "mov ds, bx");
    }

    // unset protected mode bit again
    write_cr0(cr0);

    unsafe {
        asm!("mov ds, {0:x}", in(reg) ds, options(nostack, preserves_flags));
        asm!("sti");
    }
}

#[no_mangle]
pub unsafe fn copy_to_protected_mode(target: *mut u8, bytes: &[u8]) {
    for (offset, byte) in bytes.iter().enumerate() {
        let dst = target.wrapping_add(offset);
        // we need to do the write in inline assembly because the compiler
        // seems to truncate the address
        unsafe { asm!("mov [{}], {}", in(reg) dst, in(reg_byte) *byte) };
        assert_eq!(read_from_protected_mode(dst), *byte);
    }
}

#[no_mangle]
pub unsafe fn read_from_protected_mode(ptr: *mut u8) -> u8 {
    let res;
    // we need to do the read in inline assembly because the compiler
    // seems to truncate the address
    unsafe { asm!("mov {}, [{}]", out(reg_byte) res, in(reg) ptr) };
    res
}

pub fn enter_protected_mode_and_jump_to_stage_3(entry_point: *const u8, info: &mut BiosInfo) {
    unsafe { asm!("cli") };
    set_protected_mode_bit();
    unsafe {
        asm!(
            // align the stack
            "and esp, 0xffffff00",
            // push arguments
            "push {info:e}",
            // push entry point address
            "push {entry_point:e}",
            info = in(reg) info as *const _ as u32,
            entry_point = in(reg) entry_point as u32,
        );
        asm!("ljmp $0x8, $2f", "2:", options(att_syntax));
        asm!(
            ".code32",

            // reload segment registers
            "mov bx, 0x10",
            "mov ds, bx",
            "mov es, bx",
            "mov ss, bx",

            // jump to third stage
            "pop eax",
            "call eax",

            // enter endless loop in case third stage returns
            "2:",
            "jmp 2b",
            out("eax") _
        );
    }
}

fn set_protected_mode_bit() -> u32 {
    let mut cr0: u32;
    unsafe {
        asm!("mov {:e}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags));
    }
    let cr0_protected = cr0 | 1;
    write_cr0(cr0_protected);
    cr0
}

fn write_cr0(val: u32) {
    unsafe { asm!("mov cr0, {:e}", in(reg) val, options(nostack, preserves_flags)) };
}
