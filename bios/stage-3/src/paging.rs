use bootloader_x86_64_bios_common::racy_cell::RacyCell;
use core::arch::asm;

static LEVEL_4: RacyCell<PageTable> = RacyCell::new(PageTable::empty());
static LEVEL_3: RacyCell<PageTable> = RacyCell::new(PageTable::empty());
static LEVEL_2: RacyCell<[PageTable; 10]> = RacyCell::new([PageTable::empty(); 10]);

pub fn init() {
    create_mappings();

    enable_paging();
}

fn create_mappings() {
    let l4 = unsafe { LEVEL_4.get_mut() };
    let l3 = unsafe { LEVEL_3.get_mut() };
    let l2s = unsafe { LEVEL_2.get_mut() };
    let common_flags = 0b11; // PRESENT | WRITEABLE
    l4.entries[0] = (l3 as *mut PageTable as u64) | common_flags;
    for (i, l2) in l2s.iter_mut().enumerate() {
        l3.entries[i] = (l2 as *mut PageTable as u64) | common_flags;
        let offset = u64::try_from(i).unwrap() * 1024 * 1024 * 1024;
        for (j, entry) in l2.entries.iter_mut().enumerate() {
            // map huge pages
            *entry =
                (offset + u64::try_from(j).unwrap() * (2 * 1024 * 1024)) | common_flags | (1 << 7);
        }
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

#[derive(Clone, Copy)]
#[repr(align(4096))]
struct PageTable {
    pub entries: [u64; 512],
}

impl PageTable {
    pub const fn empty() -> Self {
        Self { entries: [0; 512] }
    }
}
