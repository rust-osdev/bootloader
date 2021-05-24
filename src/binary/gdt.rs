use x86_64::{
    instructions::segmentation,
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable},
        paging::PhysFrame,
    },
    VirtAddr,
};

pub fn create_and_load(frame: PhysFrame) {
    let phys_addr = frame.start_address();
    log::info!("Creating GDT at {:?}", phys_addr);
    let virt_addr = VirtAddr::new(phys_addr.as_u64()); // utilize identity mapping

    let ptr: *mut GlobalDescriptorTable = virt_addr.as_mut_ptr();

    let mut gdt = GlobalDescriptorTable::new();
    let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
    let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
    let gdt = unsafe {
        ptr.write(gdt);
        &*ptr
    };

    gdt.load();
    unsafe {
        segmentation::set_cs(code_selector);
        segmentation::load_ds(data_selector);
        segmentation::load_es(data_selector);
        segmentation::load_ss(data_selector);
    }
}
