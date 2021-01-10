/// Allows configuring the bootloader behavior.
///
/// To control these, use a `[package.metadata.bootloader]` table in the `Cargo.toml` of
/// your kernel.
#[derive(Debug)]
pub struct Config {
    /// Whether to create a virtual mapping of the complete physical memory.
    ///
    /// Defaults to `false`.
    pub map_physical_memory: bool,
    /// Map the physical memory at a specified virtual address.
    ///
    /// If not given, the bootloader searches for a free virtual address dynamically.
    ///
    /// Only considered if `map_physical_memory` is `true`.
    pub physical_memory_offset: Option<u64>,
    /// Whether to create a recursive entry in the level 4 page table.
    ///
    /// Defaults to `false`.
    pub map_page_table_recursively: bool,
    /// Create the recursive mapping in at the given entry of the level 4 page table.
    /// 
    /// If not given, the bootloader searches for a free level 4 entry dynamically.
    ///
    /// Only considered if `map_page_table_recursively` is `true`.
    pub recursive_index: Option<u16>,
    /// Use the given stack size for the kernel.
    ///
    /// Defaults to at least 80KiB if not given.
    pub kernel_stack_size: Option<u64>,
    /// Create the kernel stack at the given virtual address.
    ///
    /// Looks for a free virtual memory region dynamically if not given.
    pub kernel_stack_address: Option<u64>,
    /// Create the boot information at the given virtual address.
    ///
    /// Looks for a free virtual memory region dynamically if not given.
    pub boot_info_address: Option<u64>,
    /// Whether to map the framebuffer to virtual memory.
    ///
    /// Defaults to `true`.
    pub map_framebuffer: bool,
    /// Map the framebuffer memory at the specified virtual address.
    ///
    /// If not given, the bootloader searches for a free virtual memory region dynamically.
    ///
    /// Only considered if `map_framebuffer` is `true`.
    pub framebuffer_address: Option<u64>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            map_physical_memory: false,
            map_page_table_recursively: false,
            map_framebuffer: true,
            physical_memory_offset: None,
            recursive_index: None,
            kernel_stack_address: None,
            kernel_stack_size: None,
            boot_info_address: None,
            framebuffer_address: None,
        }
    }
}
