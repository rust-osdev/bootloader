//! Provides boot information to the kernel.

#![deny(improper_ctypes)]

pub use self::memory_map::*;

mod memory_map;

/// This structure represents the information that the bootloader passes to the kernel.
///
/// The information is passed as an argument to the entry point:
///
/// ```ignore
/// pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
///    // […]
/// }
/// ```
///
/// Note that no type checking occurs for the entry point function, so be careful to
/// use the correct argument types. To ensure that the entry point function has the correct
/// signature, use the [`entry_point`] macro.
#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    /// A map of the physical memory regions of the underlying machine.
    ///
    /// The bootloader queries this information from the BIOS/UEFI firmware and translates this
    /// information to Rust types. It also marks any memory regions that the bootloader uses in
    /// the memory map before passing it to the kernel. Regions marked as usable can be freely
    /// used by the kernel.
    pub memory_map: MemoryMap,
    /// The virtual address of the recursively mapped level 4 page table.
    #[cfg(feature = "recursive_page_table")]
    pub recursive_page_table_addr: u64,
    /// The offset into the virtual address space where the physical memory is mapped.
    ///
    /// Physical addresses can be converted to virtual addresses by adding this offset to them.
    ///
    /// The mapping of the physical memory allows to access arbitrary physical frames. Accessing
    /// frames that are also mapped at other virtual addresses can easily break memory safety and
    /// cause undefined behavior. Only frames reported as `USABLE` by the memory map in the `BootInfo`
    /// can be safely accessed.
    #[cfg(feature = "map_physical_memory")]
    pub physical_memory_offset: u64,
    tls_template: TlsTemplate,
    _non_exhaustive: u8, // `()` is not FFI safe
}

impl BootInfo {
    /// Create a new boot information structure. This function is only for internal purposes.
    #[allow(unused_variables)]
    #[doc(hidden)]
    pub fn new(
        memory_map: MemoryMap,
        tls_template: Option<TlsTemplate>,
        recursive_page_table_addr: u64,
        physical_memory_offset: u64,
    ) -> Self {
        let tls_template = tls_template.unwrap_or(TlsTemplate {
            start_addr: 0,
            file_size: 0,
            mem_size: 0,
        });
        BootInfo {
            memory_map,
            tls_template,
            #[cfg(feature = "recursive_page_table")]
            recursive_page_table_addr,
            #[cfg(feature = "map_physical_memory")]
            physical_memory_offset,
            _non_exhaustive: 0,
        }
    }

    /// Returns information about the thread local storage segment of the kernel.
    ///
    /// Returns `None` if the kernel has no thread local storage segment.
    ///
    /// (The reason this is a method instead of a normal field is that `Option`
    /// is not FFI-safe.)
    pub fn tls_template(&self) -> Option<TlsTemplate> {
        if self.tls_template.mem_size > 0 {
            Some(self.tls_template)
        } else {
            None
        }
    }

    /// Returns the index into the page tables that recursively maps the page tables themselves.
    #[cfg(feature = "recursive_page_table")]
    pub fn recursive_index(&self) -> u16 {
        ((self.recursive_page_table_addr >> 12) & 0x1FF) as u16
    }
}

/// Information about the thread local storage (TLS) template.
///
/// This template can be used to set up thread local storage for threads. For
/// each thread, a new memory location of size `mem_size` must be initialized.
/// Then the first `file_size` bytes of this template needs to be copied to the
/// location. The additional `mem_size - file_size` bytes must be initialized with
/// zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
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

extern "C" {
    fn _improper_ctypes_check_bootinfo(_boot_info: BootInfo);
}
