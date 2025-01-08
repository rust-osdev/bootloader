#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

use crate::memory_descriptor::UefiMemoryDescriptor;
use bootloader_api::info::FrameBufferInfo;
use bootloader_boot_config::BootConfig;
use bootloader_x86_64_common::{
    legacy_memory_region::LegacyFrameAllocator, Kernel, RawFrameBufferInfo, SystemInfo,
};
use core::{
    ops::{Deref, DerefMut},
    ptr, slice,
    sync::atomic::AtomicBool,
};
use uefi::{
    boot::{
        allocate_pages, exit_boot_services, get_handle_for_protocol, image_handle,
        locate_device_path, open_protocol, ScopedProtocol,
    },
    mem::memory_map::{MemoryMap, MemoryMapMut},
    prelude::{entry, Handle, Status},
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
        ProtocolPointer,
    },
    system::with_config_table,
    table::boot::{AllocateType, MemoryType, OpenProtocolAttributes, OpenProtocolParams},
    CStr16, CStr8,
};
use x86_64::{
    structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

mod memory_descriptor;

static SYSTEM_SERVICE_AVAILABLE: AtomicBool = AtomicBool::new(true);

#[entry]
fn efi_main() -> Status {
    main_inner()
}

fn main_inner() -> Status {
    let mut boot_mode = BootMode::Disk;
    let image = image_handle();

    let mut kernel = load_kernel(image, boot_mode);
    if kernel.is_none() {
        // Try TFTP boot
        boot_mode = BootMode::Tftp;
        kernel = load_kernel(image, boot_mode);
    }
    let kernel = kernel.expect("Failed to load kernel");

    let config_file = load_config_file(image, boot_mode);
    let mut error_loading_config: Option<serde_json_core::de::Error> = None;
    let mut config: BootConfig = match config_file
        .as_deref()
        .map(serde_json_core::from_slice)
        .transpose()
    {
        Ok(data) => data.unwrap_or_default().0,
        Err(err) => {
            error_loading_config = Some(err);
            Default::default()
        }
    };

    #[allow(deprecated)] // allow writing the kernel config framebuffer
    if config.frame_buffer.minimum_framebuffer_height.is_none() {
        config.frame_buffer.minimum_framebuffer_height =
            kernel.config.frame_buffer.minimum_framebuffer_height;
    }
    #[allow(deprecated)] // allow writing the kernel config framebuffer
    if config.frame_buffer.minimum_framebuffer_width.is_none() {
        config.frame_buffer.minimum_framebuffer_width =
            kernel.config.frame_buffer.minimum_framebuffer_width;
    }
    let framebuffer = init_logger(image, &config);
    log::info!("UEFI bootloader started");

    if let Some(framebuffer) = framebuffer {
        log::info!("Using framebuffer at {:#x}", framebuffer.addr);
    }

    if let Some(err) = error_loading_config {
        log::warn!("Failed to deserialize the config file {:?}", err);
    } else {
        log::info!("Reading configuration from disk was successful");
    }

    log::info!("Trying to load ramdisk via {:?}", boot_mode);
    // Ramdisk must load from same source, or not at all.
    let ramdisk = load_ramdisk(image, boot_mode);

    log::info!(
        "{}",
        match ramdisk {
            Some(_) => "Loaded ramdisk",
            None => "Ramdisk not found.",
        }
    );

    log::trace!("exiting boot services");
    SYSTEM_SERVICE_AVAILABLE.store(false, core::sync::atomic::Ordering::SeqCst);
    let mut memory_map = unsafe { exit_boot_services(MemoryType::LOADER_DATA) };
    log::trace!("exit boot service done");

    memory_map.sort();

    let mut frame_allocator =
        LegacyFrameAllocator::new(memory_map.entries().copied().map(UefiMemoryDescriptor));

    let max_phys_addr = frame_allocator.max_phys_addr();
    let page_tables = create_page_tables(&mut frame_allocator, max_phys_addr, framebuffer.as_ref());
    let mut ramdisk_len = 0u64;
    let ramdisk_addr = if let Some(rd) = ramdisk {
        ramdisk_len = rd.len() as u64;
        Some(rd.as_ptr() as usize as u64)
    } else {
        None
    };
    let system_info = SystemInfo {
        framebuffer,
        rsdp_addr: {
            use uefi::table::cfg;
            with_config_table(|config_table| {
                let mut config_entries = config_table.iter();
                // look for an ACPI2 RSDP first
                let acpi2_rsdp = config_entries.find(|entry| matches!(entry.guid, cfg::ACPI2_GUID));
                // if no ACPI2 RSDP is found, look for a ACPI1 RSDP
                let rsdp = acpi2_rsdp
                    .or_else(|| config_entries.find(|entry| matches!(entry.guid, cfg::ACPI_GUID)));
                rsdp.map(|entry| PhysAddr::new(entry.address as u64))
            })
        },
        ramdisk_addr,
        ramdisk_len,
    };

    bootloader_x86_64_common::load_and_switch_to_kernel(
        kernel,
        config,
        frame_allocator,
        page_tables,
        system_info,
    );
}

#[derive(Clone, Copy, Debug)]
pub enum BootMode {
    Disk,
    Tftp,
}

fn load_ramdisk(image: Handle, boot_mode: BootMode) -> Option<&'static mut [u8]> {
    load_file_from_boot_method(image, "ramdisk\0", boot_mode)
}

fn load_config_file(image: Handle, boot_mode: BootMode) -> Option<&'static mut [u8]> {
    load_file_from_boot_method(image, "boot.json\0", boot_mode)
}

fn load_kernel(image: Handle, boot_mode: BootMode) -> Option<Kernel<'static>> {
    let kernel_slice = load_file_from_boot_method(image, "kernel-x86_64\0", boot_mode)?;
    Some(Kernel::parse(kernel_slice))
}

fn load_file_from_boot_method(
    image: Handle,
    filename: &str,
    boot_mode: BootMode,
) -> Option<&'static mut [u8]> {
    match boot_mode {
        BootMode::Disk => load_file_from_disk(filename, image),
        BootMode::Tftp => load_file_from_tftp_boot_server(filename, image),
    }
}

fn open_device_path_protocol(image: Handle) -> Option<ScopedProtocol<DevicePath>> {
    let loaded_image = unsafe {
        open_protocol::<LoadedImage>(
            OpenProtocolParams {
                handle: image,
                agent: image,
                controller: None,
            },
            OpenProtocolAttributes::Exclusive,
        )
    };

    if loaded_image.is_err() {
        log::error!("Failed to open protocol LoadedImage");
        return None;
    }
    let loaded_image = loaded_image.unwrap();
    let loaded_image = loaded_image.deref();

    let device_handle = loaded_image.device()?;

    let device_path = unsafe {
        open_protocol::<DevicePath>(
            OpenProtocolParams {
                handle: device_handle,
                agent: image,
                controller: None,
            },
            OpenProtocolAttributes::Exclusive,
        )
    };
    if device_path.is_err() {
        log::error!("Failed to open protocol DevicePath");
        return None;
    }
    Some(device_path.unwrap())
}

fn locate_and_open_protocol<P: ProtocolPointer>(image: Handle) -> Option<ScopedProtocol<P>> {
    let device_path = open_device_path_protocol(image)?;
    let mut device_path = device_path.deref();

    let fs_handle = locate_device_path::<P>(&mut device_path);
    if fs_handle.is_err() {
        log::error!("Failed to open device path");
        return None;
    }

    let fs_handle = fs_handle.unwrap();

    let opened_handle = unsafe {
        open_protocol::<P>(
            OpenProtocolParams {
                handle: fs_handle,
                agent: image,
                controller: None,
            },
            OpenProtocolAttributes::Exclusive,
        )
    };

    if opened_handle.is_err() {
        log::error!("Failed to open protocol {}", core::any::type_name::<P>());
        return None;
    }
    Some(opened_handle.unwrap())
}

fn load_file_from_disk(name: &str, image: Handle) -> Option<&'static mut [u8]> {
    let mut file_system_raw = locate_and_open_protocol::<SimpleFileSystem>(image)?;
    let file_system = file_system_raw.deref_mut();

    let mut root = file_system.open_volume().unwrap();
    let mut buf = [0u16; 256];
    assert!(name.len() < 256);
    let filename = CStr16::from_str_with_buf(name.trim_end_matches('\0'), &mut buf)
        .expect("Failed to convert string to utf16");

    let file_handle_result = root.open(filename, FileMode::Read, FileAttribute::empty());

    let file_handle = match file_handle_result {
        Err(_) => return None,
        Ok(handle) => handle,
    };

    let mut file = match file_handle.into_type().unwrap() {
        uefi::proto::media::file::FileType::Regular(f) => f,
        uefi::proto::media::file::FileType::Dir(_) => panic!(),
    };

    let mut buf = [0; 500];
    let file_info: &mut FileInfo = file.get_info(&mut buf).unwrap();
    let file_size = usize::try_from(file_info.file_size()).unwrap();

    let file_ptr = allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        ((file_size - 1) / 4096) + 1,
    )
    .unwrap()
    .as_ptr();
    unsafe { ptr::write_bytes(file_ptr, 0, file_size) };
    let file_slice = unsafe { slice::from_raw_parts_mut(file_ptr, file_size) };
    file.read(file_slice).unwrap();

    Some(file_slice)
}

/// Try to load a kernel from a TFTP boot server.
fn load_file_from_tftp_boot_server(name: &str, image: Handle) -> Option<&'static mut [u8]> {
    let mut base_code_raw = locate_and_open_protocol::<BaseCode>(image)?;
    let base_code = base_code_raw.deref_mut();

    // Find the TFTP boot server.
    let mode = base_code.mode();
    assert!(mode.dhcp_ack_received);
    let dhcpv4: &DhcpV4Packet = mode.dhcp_ack.as_ref();
    let server_ip = IpAddress::new_v4(dhcpv4.bootp_si_addr);
    assert!(name.len() < 256);

    let filename = CStr8::from_bytes_with_nul(name.as_bytes()).unwrap();

    // Determine the kernel file size.
    let file_size = base_code.tftp_get_file_size(&server_ip, filename).ok()?;
    let kernel_size = usize::try_from(file_size).expect("The file size should fit into usize");

    // Allocate some memory for the kernel file.
    let ptr = allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        ((kernel_size - 1) / 4096) + 1,
    )
    .expect("Failed to allocate memory for the file")
    .as_ptr();
    let slice = unsafe { slice::from_raw_parts_mut(ptr, kernel_size) };

    // Load the kernel file.
    base_code
        .tftp_read_file(&server_ip, filename, Some(slice))
        .expect("Failed to read kernel file from the TFTP boot server");

    Some(slice)
}

/// Creates page table abstraction types for both the bootloader and kernel page tables.
fn create_page_tables(
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    max_phys_addr: PhysAddr,
    frame_buffer: Option<&RawFrameBufferInfo>,
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

        // copy the pml4 entries for all identity mapped memory.
        let end_addr = VirtAddr::new(max_phys_addr.as_u64() - 1);
        for p4 in 0..=usize::from(end_addr.p4_index()) {
            new_table[p4] = old_table[p4].clone();
        }

        // copy the pml4 entry for the frame buffer (the frame buffer is not
        // necessarily part of the identity mapping).
        if let Some(frame_buffer) = frame_buffer {
            let start_addr = VirtAddr::new(frame_buffer.addr.as_u64());
            let end_addr = start_addr + frame_buffer.info.byte_len;
            for p4 in usize::from(start_addr.p4_index())..=usize::from(end_addr.p4_index()) {
                new_table[p4] = old_table[p4].clone();
            }
        }

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

fn init_logger(image_handle: Handle, config: &BootConfig) -> Option<RawFrameBufferInfo> {
    let gop_handle = get_handle_for_protocol::<GraphicsOutput>().ok()?;
    let mut gop = unsafe {
        open_protocol::<GraphicsOutput>(
            OpenProtocolParams {
                handle: gop_handle,
                agent: image_handle,
                controller: None,
            },
            OpenProtocolAttributes::Exclusive,
        )
        .ok()?
    };

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

    bootloader_x86_64_common::init_logger(
        slice,
        info,
        config.log_level,
        config.frame_buffer_logging,
        config.serial_logging,
    );

    Some(RawFrameBufferInfo {
        addr: PhysAddr::new(framebuffer.as_mut_ptr() as u64),
        info,
    })
}

#[cfg(target_os = "uefi")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::arch::asm;
    use core::fmt::Write;
    use uefi::system::with_stdout;

    if SYSTEM_SERVICE_AVAILABLE.load(core::sync::atomic::Ordering::SeqCst) {
        // this panics after the exit_boot_services call
        with_stdout(|stdout| {
            let _ = stdout.clear();
            let _ = writeln!(stdout, "{}", info);
        });
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
