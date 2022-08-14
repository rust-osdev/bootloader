use core::{arch::asm, cell::UnsafeCell};

static LEVEL_4: RacyCell<PageTable> = RacyCell::new(PageTable::empty());
static LEVEL_3: RacyCell<PageTable> = RacyCell::new(PageTable::empty());
static LEVEL_2: RacyCell<PageTable> = RacyCell::new(PageTable::empty());

pub struct RacyCell<T>(UnsafeCell<T>);

impl<T> RacyCell<T> {
    const fn new(v: T) -> Self {
        Self(UnsafeCell::new(v))
    }

    pub unsafe fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}

unsafe impl<T> Send for RacyCell<T> where T: Send {}
unsafe impl<T> Sync for RacyCell<T> {}

pub fn init() {
    create_mappings();

    enable_paging();
}

fn create_mappings() {
    let l4 = unsafe { LEVEL_4.get_mut() };
    let l3 = unsafe { LEVEL_3.get_mut() };
    let l2 = unsafe { LEVEL_2.get_mut() };
    let common_flags = 0b11;
    l4.entries[0] = (l3 as *mut PageTable as u64) | common_flags;
    l3.entries[0] = (l2 as *mut PageTable as u64) | common_flags;
    for i in 0..512 {
        l2.entries[i] = (u64::try_from(i).unwrap() * (2 * 1024 * 1024)) | common_flags | (1 << 7);
    }
}

fn enable_paging() {
    // load level 4 table pointer into cr3 register
    let l4 = unsafe { LEVEL_4.get_mut() } as *mut PageTable;
    unsafe { asm!("mov cr3, {0}", in(reg) l4) };

    // enable PAE-flag in cr4 (Physical Address Extension)
    unsafe { asm!("mov eax, cr4", "or eax, 1<<5", "mov cr4, eax", out("eax")_) };

    // set the long mode bit in the EFER MSR (model specific register)
    unsafe {
        asm!("mov ecx, 0xC0000080", "rdmsr", "or eax, 1 << 8", "wrmsr", out("eax") _, out("ecx")_)
    };

    // enable paging in the cr0 register
    unsafe { asm!("mov eax, cr0", "or eax, 1 << 31", "mov cr0, eax", out("eax")_) };
}

#[repr(align(4096))]
struct PageTable {
    pub entries: [u64; 512],
}

impl PageTable {
    pub const fn empty() -> Self {
        Self { entries: [0; 512] }
    }
}
