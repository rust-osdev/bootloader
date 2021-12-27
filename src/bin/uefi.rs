#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![feature(maybe_uninit_extra)]
#![deny(unsafe_op_in_unsafe_fn)]

#[repr(align(4096))]
struct PageAligned<T>(T);

use bootloader::binary::{legacy_memory_region::LegacyFrameAllocator, Kernel, SystemInfo};
use bootloader_api::{info::FrameBufferInfo, BootloaderConfig};
use core::{arch::asm, mem, panic::PanicInfo, ptr, slice};
use uefi::{
    prelude::{entry, Boot, Handle, ResultExt, Status, SystemTable},
    proto::{
        console::gop::{GraphicsOutput, PixelFormat},
        device_path::DevicePath,
        loaded_image::LoadedImage,
        media::{
            file::{File, FileAttribute, FileInfo, FileMode},
            fs::SimpleFileSystem,
        },
    },
    table::boot::{AllocateType, MemoryDescriptor, MemoryType},
    CStr16, Completion,
};
use x86_64::{
    structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};
use xmas_elf::ElfFile;

#[entry]
fn efi_main(image: Handle, st: SystemTable<Boot>) -> Status {
    main_inner(image, st)
}

fn main_inner(image: Handle, mut st: SystemTable<Boot>) -> Status {
    let mut buf = [0; 100];
    st.stdout()
        .output_string(
            CStr16::from_str_with_buf("UEFI bootloader started; trying to load kernel", &mut buf)
                .unwrap(),
        )
        .unwrap()
        .unwrap();

    let file_system_raw = {
        let ref this = st.boot_services();
        let loaded_image = this
            .handle_protocol::<LoadedImage>(image)?
            .expect("Failed to retrieve `LoadedImage` protocol from handle");
        let loaded_image = unsafe { &*loaded_image.get() };

        let device_handle = loaded_image.device();

        let device_path = this
            .handle_protocol::<DevicePath>(device_handle)?
            .expect("Failed to retrieve `DevicePath` protocol from image's device handle");
        let mut device_path = unsafe { &*device_path.get() };

        let device_handle = this
            .locate_device_path::<SimpleFileSystem>(&mut device_path)?
            .expect("Failed to locate `SimpleFileSystem` protocol on device path");

        this.handle_protocol::<SimpleFileSystem>(device_handle)
    }
    .unwrap()
    .unwrap();
    let file_system = unsafe { &mut *file_system_raw.get() };

    let mut root = file_system.open_volume().unwrap().unwrap();
    let kernel_file_handle = root
        .open("kernel-x86_64", FileMode::Read, FileAttribute::empty())
        .unwrap()
        .unwrap();
    let mut kernel_file = match kernel_file_handle.into_type().unwrap().unwrap() {
        uefi::proto::media::file::FileType::Regular(f) => f,
        uefi::proto::media::file::FileType::Dir(_) => panic!(),
    };

    let mut buf = [0; 100];
    let kernel_info: &mut FileInfo = kernel_file.get_info(&mut buf).unwrap().unwrap();
    let kernel_size = usize::try_from(kernel_info.file_size()).unwrap();

    let kernel_ptr = st
        .boot_services()
        .allocate_pages(
            AllocateType::AnyPages,
            MemoryType::LOADER_DATA,
            ((kernel_size - 1) / 4096) + 1,
        )
        .unwrap()
        .unwrap() as *mut u8;
    unsafe { ptr::write_bytes(kernel_ptr, 0, kernel_size) };
    let kernel_slice = unsafe { slice::from_raw_parts_mut(kernel_ptr, kernel_size) };
    kernel_file.read(kernel_slice).unwrap().unwrap();

    let kernel_elf = ElfFile::new(kernel_slice).unwrap();

    let config = {
        let section = kernel_elf
            .find_section_by_name(".bootloader-config")
            .unwrap();
        let raw = section.raw_data(&kernel_elf);
        BootloaderConfig::deserialize(raw).unwrap()
    };

    let kernel = Kernel {
        elf: kernel_elf,
        config,
    };

    let (framebuffer_addr, framebuffer_info) = init_logger(&st, config);
    log::info!("UEFI bootloader started");
    log::info!("Reading kernel and configuration from disk was successful");
    log::info!("Using framebuffer at {:#x}", framebuffer_addr);

    let mmap_storage = {
        let max_mmap_size =
            st.boot_services().memory_map_size() + 8 * mem::size_of::<MemoryDescriptor>();
        let ptr = st
            .boot_services()
            .allocate_pool(MemoryType::LOADER_DATA, max_mmap_size)?
            .log();
        unsafe { slice::from_raw_parts_mut(ptr, max_mmap_size) }
    };

    log::trace!("exiting boot services");
    let (system_table, memory_map) = st
        .exit_boot_services(image, mmap_storage)
        .expect_success("Failed to exit boot services");

    let mut frame_allocator = LegacyFrameAllocator::new(memory_map.copied());

    let page_tables = create_page_tables(&mut frame_allocator);

    let system_info = SystemInfo {
        framebuffer_addr,
        framebuffer_info,
        rsdp_addr: {
            use uefi::table::cfg;
            let mut config_entries = system_table.config_table().iter();
            // look for an ACPI2 RSDP first
            let acpi2_rsdp = config_entries.find(|entry| matches!(entry.guid, cfg::ACPI2_GUID));
            // if no ACPI2 RSDP is found, look for a ACPI1 RSDP
            let rsdp = acpi2_rsdp
                .or_else(|| config_entries.find(|entry| matches!(entry.guid, cfg::ACPI_GUID)));
            rsdp.map(|entry| PhysAddr::new(entry.address as u64))
        },
    };

    bootloader::binary::load_and_switch_to_kernel(
        kernel,
        frame_allocator,
        page_tables,
        system_info,
    );
}

/// Creates page table abstraction types for both the bootloader and kernel page tables.
fn create_page_tables(
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> bootloader::binary::PageTables {
    // UEFI identity-maps all memory, so the offset between physical and virtual addresses is 0
    let phys_offset = VirtAddr::new(0);

    // copy the currently active level 4 page table, because it might be read-only
    log::trace!("switching to new level 4 table");
    let bootloader_page_table = {
        let old_table = {
            let frame = x86_64::registers::control::Cr3::read().0;
            let ptr: *const PageTable = (phys_offset + frame.start_address().as_u64()).as_ptr();
            unsafe { &*ptr }
        };
        let new_frame = frame_allocator
            .allocate_frame()
            .expect("Failed to allocate frame for new level 4 table");
        let new_table: &mut PageTable = {
            let ptr: *mut PageTable =
                (phys_offset + new_frame.start_address().as_u64()).as_mut_ptr();
            // create a new, empty page table
            unsafe {
                ptr.write(PageTable::new());
                &mut *ptr
            }
        };

        // copy the first entry (we don't need to access more than 512 GiB; also, some UEFI
        // implementations seem to create an level 4 table entry 0 in all slots)
        new_table[0] = old_table[0].clone();

        // the first level 4 table entry is now identical, so we can just load the new one
        unsafe {
            x86_64::registers::control::Cr3::write(
                new_frame,
                x86_64::registers::control::Cr3Flags::empty(),
            );
            OffsetPageTable::new(&mut *new_table, phys_offset)
        }
    };

    // create a new page table hierarchy for the kernel
    let (kernel_page_table, kernel_level_4_frame) = {
        // get an unused frame for new level 4 page table
        let frame: PhysFrame = frame_allocator.allocate_frame().expect("no unused frames");
        log::info!("New page table at: {:#?}", &frame);
        // get the corresponding virtual address
        let addr = phys_offset + frame.start_address().as_u64();
        // initialize a new page table
        let ptr = addr.as_mut_ptr();
        unsafe { *ptr = PageTable::new() };
        let level_4_table = unsafe { &mut *ptr };
        (
            unsafe { OffsetPageTable::new(level_4_table, phys_offset) },
            frame,
        )
    };

    bootloader::binary::PageTables {
        bootloader: bootloader_page_table,
        kernel: kernel_page_table,
        kernel_level_4_frame,
    }
}

fn init_logger(st: &SystemTable<Boot>, config: BootloaderConfig) -> (PhysAddr, FrameBufferInfo) {
    let gop = st
        .boot_services()
        .locate_protocol::<GraphicsOutput>()
        .expect_success("failed to locate gop");
    let gop = unsafe { &mut *gop.get() };

    let mode = {
        let modes = gop.modes().map(Completion::unwrap);
        match (
            config
                .frame_buffer
                .minimum_framebuffer_height
                .map(|v| usize::try_from(v).unwrap()),
            config
                .frame_buffer
                .minimum_framebuffer_width
                .map(|v| usize::try_from(v).unwrap()),
        ) {
            (Some(height), Some(width)) => modes
                .filter(|m| {
                    let res = m.info().resolution();
                    res.1 >= height && res.0 >= width
                })
                .last(),
            (Some(height), None) => modes.filter(|m| m.info().resolution().1 >= height).last(),
            (None, Some(width)) => modes.filter(|m| m.info().resolution().0 >= width).last(),
            _ => None,
        }
    };
    if let Some(mode) = mode {
        gop.set_mode(&mode)
            .expect_success("Failed to apply the desired display mode");
    }

    let mode_info = gop.current_mode_info();
    let mut framebuffer = gop.frame_buffer();
    let slice = unsafe { slice::from_raw_parts_mut(framebuffer.as_mut_ptr(), framebuffer.size()) };
    let info = FrameBufferInfo {
        byte_len: framebuffer.size(),
        horizontal_resolution: mode_info.resolution().0,
        vertical_resolution: mode_info.resolution().1,
        pixel_format: match mode_info.pixel_format() {
            PixelFormat::Rgb => bootloader_api::info::PixelFormat::Rgb,
            PixelFormat::Bgr => bootloader_api::info::PixelFormat::Bgr,
            PixelFormat::Bitmask | PixelFormat::BltOnly => {
                panic!("Bitmask and BltOnly framebuffers are not supported")
            }
        },
        bytes_per_pixel: 4,
        stride: mode_info.stride(),
    };

    log::info!("UEFI boot");

    bootloader::binary::init_logger(slice, info);

    (PhysAddr::new(framebuffer.as_mut_ptr() as u64), info)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        bootloader::binary::logger::LOGGER
            .get()
            .map(|l| l.force_unlock())
    };
    log::error!("{}", info);
    loop {
        unsafe { asm!("cli; hlt") };
    }
}
