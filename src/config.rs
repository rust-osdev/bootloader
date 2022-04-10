#[cfg(feature = "binary")]
const PAGE_SIZE: u64 = 4096;

/// Allows configuring the bootloader behavior.
///
/// To control these, use a `[package.metadata.bootloader]` table in the `Cargo.toml` of
/// your kernel. The naming convention for all config fields is `kebab-case`, otherwise the
/// config keys correspond to the field names of this struct (i.e. just replace `_` with `-`).
/// Unknown config keys lead to an error.
///
/// ## Example
///
/// To map the complete physical memory starting at virtual address `0x0000_4000_0000_0000`, add
/// the following to your kernel's `Cargo.toml`:
///
/// ```toml
/// [package.metadata.bootloader]
/// map-physical-memory = true
/// physical-memory-offset = 0x0000_4000_0000_0000
/// ```
///
/// ## Memory Addresses
///
/// Memory addresses must be positive and page aligned. Since TOML does not support unsigned 64-bit
/// integers, we also support string input to specify addresses larger than `i64::MAX`. For example:
///
/// ```toml
/// physical-memory-offset = "0xf000_0000_0000_0000"
/// ```
///
/// The above example would fail if the address was specified as integer instead (i.e. without
/// the quotes).
///
/// All memory addresses are optional, even if their corresponding switch is enabled. If no
/// address is specified, the bootloader will choose an unused entry of the level 4 page table
/// at runtime. The addresses can be restricted by setting the `dynamic-range` configuration key.
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
    /// Whether to randomize non-statically configured addresses.
    /// The kernel base address will be randomized when it's compiled as
    /// a position independent executable.
    ///
    /// Defaults to `false`.
    pub aslr: bool,
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
    /// Desired minimum height of the framebuffer mode.
    ///
    /// Defaults to using the default mode if neither `minimum_framebuffer_height` or
    /// `minimum_framebuffer_width` is supplied, and using the last available mode that
    /// fits them if 1 or more is set.
    pub minimum_framebuffer_height: Option<usize>,
    /// Desired minimum width of the framebuffer mode.
    ///
    /// Defaults to using the default mode if neither `minimum_framebuffer_height` or
    /// `minimum_framebuffer_width` is supplied, and using the last available mode that
    /// fits them if 1 or more is set.
    pub minimum_framebuffer_width: Option<usize>,
    /// The lowest virtual address for dynamic addresses.
    ///
    /// Defaults to `0`.
    pub dynamic_range_start: Option<u64>,
    /// The highest virtual address for dynamic addresses.
    ///
    /// Defaults to `0xffff_ffff_ffff_f000`.
    pub dynamic_range_end: Option<u64>,
}

#[cfg(feature = "binary")]
impl Config {
    pub(crate) fn kernel_stack_size(&self) -> u64 {
        self.kernel_stack_size.unwrap_or(20 * PAGE_SIZE)
    }
}
