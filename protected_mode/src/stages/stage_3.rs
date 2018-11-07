use x86_64::{
    ux::u9,
    PhysAddr,
    structures::paging::{PageTable, PageTableFlags, PhysFrame, Size2MiB},
};


extern {
    static mut _p4: PageTable;
    static mut _p3: PageTable;
    static mut _p2: PageTable;
}

#[no_mangle]
extern "C" fn stage_3() -> ! {
    // Identity mapping of first gigabyte + recursive mapping of P4 table
    /*unsafe {
        let p4_addr = PhysAddr::new(&P4 as *const PageTable as u64);
        let p3_addr = PhysAddr::new(&P3 as *const PageTable as u64);
        let p2_addr = PhysAddr::new(&P2 as *const PageTable as u64);

        P4[0].set_addr(p3_addr, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
        P4[511].set_addr(p4_addr, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
        P3[0].set_addr(p2_addr, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
        {
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::HUGE_PAGE;
            let start_frame = PhysFrame::<Size2MiB>::containing_address(PhysAddr::new(0));
            for i in 0u16..512 {
                P2[u9::new(i)].set_addr((start_frame + i.into()).start_address(), flags);
            }
        }
    }
    */

    use core::fmt::Write;
    unsafe {
    write!(crate::printer::Printer, "P4: {}, P3: {}, P2 {}", &_p4 as *const _ as u32, &_p3 as *const _ as u32, &_p2 as *const _ as u32);
    }

    let ptr = 0xb8200 as *mut u16;
    unsafe {
        *ptr = 0xffff;
    }
    loop {}
}

