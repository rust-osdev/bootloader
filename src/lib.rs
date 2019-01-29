#![no_std]

/// The virtual address of the recursively mapped level 4 page table.
#[cfg(feature = "recursive_level_4_table")]
pub const RECURSIVE_LEVEL_4_TABLE_ADDR: u64 = 0o_177777_777_777_777_777_0000;

/// The offset into the virtual address space where the physical memory is mapped.
///
/// Physical addresses can be converted to virtual addresses by adding this offset to them.
///
/// The mapping of the physical memory allows to access arbitrary physical frames. Accessing
/// frames that are also mapped at other virtual addresses can easily break memory safety and
/// cause undefined behavior. Only frames reported as `USABLE` by the memory map in the `BootInfo`
/// can be safely accessed.
#[cfg(feature = "map_physical_memory")]
pub const PHYSICAL_MEMORY_OFFSET: u64 = 0o_177777_770_000_000_000_0000;

pub use crate::bootinfo::BootInfo;

pub mod bootinfo;

/// Defines the entry point function.
///
/// The function must have the signature `fn(&'static BootInfo) -> !`.
///
/// This macro just creates a function named `_start`, which the linker will use as the entry
/// point. The advantage of using this macro instead of providing an own `_start` function is
/// that the macro ensures that the function and argument types are correct.
#[macro_export]
macro_rules! entry_point {
    ($path:path) => {
        #[cfg(not(test))]
        #[export_name = "_start"]
        pub extern "C" fn __impl_start(boot_info: &'static $crate::bootinfo::BootInfo) -> ! {
            // validate the signature of the program entry point
            let f: fn(&'static $crate::bootinfo::BootInfo) -> ! = $path;

            f(boot_info)
        }
    };
}
