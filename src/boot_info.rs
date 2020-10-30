use crate::memory_map::MemoryRegion;
use core::slice;

/// This structure represents the information that the bootloader passes to the kernel.
///
/// The information is passed as an argument to the entry point. The entry point function must
/// have the following signature:
///
/// ```ignore
/// pub extern "C" fn(boot_info: &'static BootInfo) -> !;
/// ```
///
/// Note that no type checking occurs for the entry point function, so be careful to
/// use the correct argument types. To ensure that the entry point function has the correct
/// signature, use the [`entry_point`] macro.
#[derive(Debug)]
pub struct BootInfo {
    /// A map of the physical memory regions of the underlying machine.
    ///
    /// The bootloader queries this information from the BIOS/UEFI firmware and translates this
    /// information to Rust types. It also marks any memory regions that the bootloader uses in
    /// the memory map before passing it to the kernel. Regions marked as usable can be freely
    /// used by the kernel.
    pub memory_regions: &'static mut [MemoryRegion],
    /// Information about the framebuffer for screen output if available.
    pub framebuffer: Option<FrameBuffer>,
    /// The virtual address at which the mapping of the physical memory starts.
    ///
    /// Physical addresses can be converted to virtual addresses by adding this offset to them.
    ///
    /// The mapping of the physical memory allows to access arbitrary physical frames. Accessing
    /// frames that are also mapped at other virtual addresses can easily break memory safety and
    /// cause undefined behavior. Only frames reported as `USABLE` by the memory map in the `BootInfo`
    /// can be safely accessed.
    ///
    /// Only available if the `map-physical-memory` config option is enabled.
    pub physical_memory_offset: Option<u64>,
    /// The virtual address of the recursively mapped level 4 page table.
    ///
    /// Only available if the `map-page-table-recursively` config option is enabled.
    pub recursive_index: Option<u16>,
    /// The address of the `RSDP` data structure, which can be use to find the ACPI tables.
    ///
    /// This field is `None` if no `RSDP` was found (for BIOS) or reported (for UEFI).
    pub rsdp_addr: Option<u64>,
    /// The thread local storage (TLS) template of the kernel executable, if present.
    pub tls_template: Option<TlsTemplate>,
    pub(crate) _non_exhaustive: (),
}

#[derive(Debug)]
pub struct FrameBuffer {
    pub(crate) buffer_start: u64,
    pub(crate) buffer_byte_len: usize,
    pub(crate) info: FrameBufferInfo,
}

impl FrameBuffer {
    pub fn buffer(&mut self) -> &mut [u8] {
        unsafe { self.create_buffer() }
    }

    unsafe fn create_buffer<'a>(&self) -> &'a mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.buffer_start as *mut u8, self.buffer_byte_len) }
    }

    pub fn info(&self) -> FrameBufferInfo {
        self.info
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FrameBufferInfo {
    pub byte_len: usize,
    pub horizontal_resolution: usize,
    pub vertical_resolution: usize,
    pub pixel_format: PixelFormat,
    pub bytes_per_pixel: usize,
    pub stride: usize,
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum PixelFormat {
    RGB,
    BGR,
    U8,
}

/// Information about the thread local storage (TLS) template.
///
/// This template can be used to set up thread local storage for threads. For
/// each thread, a new memory location of size `mem_size` must be initialized.
/// Then the first `file_size` bytes of this template needs to be copied to the
/// location. The additional `mem_size - file_size` bytes must be initialized with
/// zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlsTemplate {
    /// The virtual start address of the thread local storage template.
    pub start_addr: u64,
    /// The number of data bytes in the template.
    ///
    /// Corresponds to the length of the `.tdata` section.
    pub file_size: u64,
    /// The total number of bytes that the TLS segment should have in memory.
    ///
    /// Corresponds to the combined length of the `.tdata` and `.tbss` sections.
    pub mem_size: u64,
}

/// Check that the _pointer_ is FFI-safe.
///
/// Note that the `BootInfo` struct is not FFI-safe, so it needs to be compiled by the same Rust
/// compiler as the kernel in order to be safely accessed.
extern "C" fn _assert_ffi(_boot_info: &'static mut BootInfo) {}
