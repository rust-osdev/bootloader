#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![deny(unsafe_op_in_unsafe_fn)]

use crate::memory_descriptor::UefiMemoryDescriptor;
use bootloader_api::{info::FrameBufferInfo, BootloaderConfig};
use bootloader_x86_64_common::{
    legacy_memory_region::LegacyFrameAllocator, Kernel, RawFrameBufferInfo, SystemInfo,
};
use core::{cell::UnsafeCell, fmt::Write, mem, ptr, slice};
use uefi::{
    prelude::{entry, Boot, Handle, Status, SystemTable},
    proto::{
        console::gop::{GraphicsOutput, PixelFormat},
        device_path::DevicePath,
        loaded_image::LoadedImage,
        media::{
            file::{File, FileAttribute, FileInfo, FileMode},
            fs::SimpleFileSystem,
        },
        network::{
            pxe::{BaseCode, DhcpV4Packet},
            IpAddress,
        },
    },
    table::boot::{
        AllocateType, MemoryDescriptor, MemoryType, OpenProtocolAttributes, OpenProtocolParams,
    },
    CStr16, CStr8,
};
use x86_64::{
    structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

mod memory_descriptor;

static SYSTEM_TABLE: RacyCell<Option<SystemTable<Boot>>> = RacyCell::new(None);

struct RacyCell<T>(UnsafeCell<T>);

impl<T> RacyCell<T> {
    const fn new(v: T) -> Self {
        Self(UnsafeCell::new(v))
    }
}

unsafe impl<T> Sync for RacyCell<T> {}

impl<T> core::ops::Deref for RacyCell<T> {
    type Target = UnsafeCell<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[entry]
fn efi_main(image: Handle, st: SystemTable<Boot>) -> Status {
    main_inner(image, st)
}

fn main_inner(image: Handle, mut st: SystemTable<Boot>) -> Status {
    // temporarily clone the y table for printing panics
    unsafe {
        *SYSTEM_TABLE.get() = Some(st.unsafe_clone());
    }

    st.stdout().clear().unwrap();
    writeln!(
        st.stdout(),
        "UEFI bootloader started; trying to load kernel"
    )
    .unwrap();

    let kernel = load_kernel(image, &st);

    let framebuffer = init_logger(&st, kernel.config);

    // we no longer need the system table for printing panics
    unsafe {
        *SYSTEM_TABLE.get() = None;
    }

    log::info!("UEFI bootloader started");
    log::info!("Reading kernel and configuration from disk was successful");
    if let Some(framebuffer) = framebuffer {
        log::info!("Using framebuffer at {:#x}", framebuffer.addr);
    }

    let mmap_storage = {
        let max_mmap_size =
            st.boot_services().memory_map_size().map_size + 8 * mem::size_of::<MemoryDescriptor>();
        let ptr = st
            .boot_services()
            .allocate_pool(MemoryType::LOADER_DATA, max_mmap_size)?;
        unsafe { slice::from_raw_parts_mut(ptr, max_mmap_size) }
    };

    log::trace!("exiting boot services");
    let (system_table, memory_map) = st
        .exit_boot_services(image, mmap_storage)
        .expect("Failed to exit boot services");

    let mut frame_allocator =
        LegacyFrameAllocator::new(memory_map.copied().map(UefiMemoryDescriptor));

    let page_tables = create_page_tables(&mut frame_allocator);

    let system_info = SystemInfo {
        framebuffer,
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

    bootloader_x86_64_common::load_and_switch_to_kernel(
        kernel,
        frame_allocator,
        page_tables,
        system_info,
    );
}

fn load_kernel(image: Handle, st: &SystemTable<Boot>) -> Kernel<'static> {
    let kernel_slice = load_kernel_file(image, st).expect("couldn't find kernel");
    Kernel::parse(kernel_slice)
}

/// Try to load a kernel file from the boot device.
fn load_kernel_file(image: Handle, st: &SystemTable<Boot>) -> Option<&'static mut [u8]> {
    load_kernel_file_from_disk(image, st)
        .or_else(|| load_kernel_file_from_tftp_boot_server(image, st))
}

fn load_kernel_file_from_disk(image: Handle, st: &SystemTable<Boot>) -> Option<&'static mut [u8]> {
    let file_system_raw = {
        let this = st.boot_services();
        let loaded_image = this
            .open_protocol::<LoadedImage>(
                OpenProtocolParams {
                    handle: image,
                    agent: image,
                    controller: None,
                },
                OpenProtocolAttributes::Exclusive,
            )
            .expect("Failed to retrieve `LoadedImage` protocol from handle");
        let loaded_image = unsafe { &*loaded_image.interface.get() };

        let device_handle = loaded_image.device();

        let device_path = this
            .open_protocol::<DevicePath>(
                OpenProtocolParams {
                    handle: device_handle,
                    agent: image,
                    controller: None,
                },
                OpenProtocolAttributes::Exclusive,
            )
            .expect("Failed to retrieve `DevicePath` protocol from image's device handle");
        let mut device_path = unsafe { &*device_path.interface.get() };

        let fs_handle = this
            .locate_device_path::<SimpleFileSystem>(&mut device_path)
            .ok()?;

        this.open_protocol::<SimpleFileSystem>(
            OpenProtocolParams {
                handle: fs_handle,
                agent: image,
                controller: None,
            },
            OpenProtocolAttributes::Exclusive,
        )
    }
    .unwrap();
    let file_system = unsafe { &mut *file_system_raw.interface.get() };

    let mut root = file_system.open_volume().unwrap();
    let mut buf = [0; 14 * 2];
    let filename = CStr16::from_str_with_buf("kernel-x86_64", &mut buf).unwrap();
    let kernel_file_handle = root
        .open(filename, FileMode::Read, FileAttribute::empty())
        .expect("Failed to load kernel (expected file named `kernel-x86_64`)");
    let mut kernel_file = match kernel_file_handle.into_type().unwrap() {
        uefi::proto::media::file::FileType::Regular(f) => f,
        uefi::proto::media::file::FileType::Dir(_) => panic!(),
    };

    let mut buf = [0; 500];
    let kernel_info: &mut FileInfo = kernel_file.get_info(&mut buf).unwrap();
    let kernel_size = usize::try_from(kernel_info.file_size()).unwrap();

    let kernel_ptr = st
        .boot_services()
        .allocate_pages(
            AllocateType::AnyPages,
            MemoryType::LOADER_DATA,
            ((kernel_size - 1) / 4096) + 1,
        )
        .unwrap() as *mut u8;
    unsafe { ptr::write_bytes(kernel_ptr, 0, kernel_size) };
    let kernel_slice = unsafe { slice::from_raw_parts_mut(kernel_ptr, kernel_size) };
    kernel_file.read(kernel_slice).unwrap();

    Some(kernel_slice)
}

/// Try to load a kernel from a TFTP boot server.
fn load_kernel_file_from_tftp_boot_server(
    image: Handle,
    st: &SystemTable<Boot>,
) -> Option<&'static mut [u8]> {
    let this = st.boot_services();

    // Try to locate a `BaseCode` protocol on the boot device.

    let loaded_image = this
        .open_protocol::<LoadedImage>(
            OpenProtocolParams {
                handle: image,
                agent: image,
                controller: None,
            },
            OpenProtocolAttributes::Exclusive,
        )
        .expect("Failed to retrieve `LoadedImage` protocol from handle");
    let loaded_image = unsafe { &*loaded_image.interface.get() };

    let device_handle = loaded_image.device();

    let device_path = this
        .open_protocol::<DevicePath>(
            OpenProtocolParams {
                handle: device_handle,
                agent: image,
                controller: None,
            },
            OpenProtocolAttributes::Exclusive,
        )
        .expect("Failed to retrieve `DevicePath` protocol from image's device handle");
    let mut device_path = unsafe { &*device_path.interface.get() };

    let base_code_handle = this.locate_device_path::<BaseCode>(&mut device_path).ok()?;

    let base_code_raw = this
        .open_protocol::<BaseCode>(
            OpenProtocolParams {
                handle: base_code_handle,
                agent: image,
                controller: None,
            },
            OpenProtocolAttributes::Exclusive,
        )
        .unwrap();
    let base_code = unsafe { &mut *base_code_raw.interface.get() };

    // Find the TFTP boot server.
    let mode = base_code.mode();
    assert!(mode.dhcp_ack_received);
    let dhcpv4: &DhcpV4Packet = mode.dhcp_ack.as_ref();
    let server_ip = IpAddress::new_v4(dhcpv4.bootp_si_addr);

    let filename = CStr8::from_bytes_with_nul(b"kernel-x86_64\0").unwrap();

    // Determine the kernel file size.
    let file_size = base_code
        .tftp_get_file_size(&server_ip, filename)
        .expect("Failed to query the kernel file size");
    let kernel_size =
        usize::try_from(file_size).expect("The kernel file size should fit into usize");

    // Allocate some memory for the kernel file.
    let kernel_ptr = st
        .boot_services()
        .allocate_pages(
            AllocateType::AnyPages,
            MemoryType::LOADER_DATA,
            ((kernel_size - 1) / 4096) + 1,
        )
        .expect("Failed to allocate memory for the kernel file") as *mut u8;
    let kernel_slice = unsafe { slice::from_raw_parts_mut(kernel_ptr, kernel_size) };

    // Load the kernel file.
    base_code
        .tftp_read_file(&server_ip, filename, Some(kernel_slice))
        .expect("Failed to read kernel file from the TFTP boot server");

    Some(kernel_slice)
}

/// Creates page table abstraction types for both the bootloader and kernel page tables.
fn create_page_tables(
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> bootloader_x86_64_common::PageTables {
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

    bootloader_x86_64_common::PageTables {
        bootloader: bootloader_page_table,
        kernel: kernel_page_table,
        kernel_level_4_frame,
    }
}

fn init_logger(st: &SystemTable<Boot>, config: BootloaderConfig) -> Option<RawFrameBufferInfo> {
    let gop = st
        .boot_services()
        .locate_protocol::<GraphicsOutput>()
        .ok()?;
    let gop = unsafe { &mut *gop.get() };

    let mode = {
        let modes = gop.modes();
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
            .expect("Failed to apply the desired display mode");
    }

    let mode_info = gop.current_mode_info();
    let mut framebuffer = gop.frame_buffer();
    let slice = unsafe { slice::from_raw_parts_mut(framebuffer.as_mut_ptr(), framebuffer.size()) };
    let info = FrameBufferInfo {
        byte_len: framebuffer.size(),
        width: mode_info.resolution().0,
        height: mode_info.resolution().1,
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

    bootloader_x86_64_common::init_logger(slice, info);

    Some(RawFrameBufferInfo {
        addr: PhysAddr::new(framebuffer.as_mut_ptr() as u64),
        info,
    })
}

#[cfg(target_os = "uefi")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::arch::asm;

    if let Some(st) = unsafe { &mut *SYSTEM_TABLE.get() } {
        let _ = writeln!(st.stdout(), "{}", info);
    }

    unsafe {
        bootloader_x86_64_common::logger::LOGGER
            .get()
            .map(|l| l.force_unlock())
    };
    log::error!("{}", info);

    loop {
        unsafe { asm!("cli; hlt") };
    }
}
