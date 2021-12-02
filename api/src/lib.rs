#![feature(asm)]
#![feature(split_array)]

#![deny(unsafe_op_in_unsafe_fn)]

#![cfg_attr(not(test), no_std)]

pub use config::BootloaderConfig;

pub mod config;
pub mod info;

mod concat {
    include!(concat!(env!("OUT_DIR"), "/concat.rs"));
}

/// Defines the entry point function.
///
/// The function must have the signature `fn(&'static mut BootInfo) -> !`.
///
/// This macro just creates a function named `_start`, which the linker will use as the entry
/// point. The advantage of using this macro instead of providing an own `_start` function is
/// that the macro ensures that the function and argument types are correct.
#[macro_export]
macro_rules! entry_point {
    ($path:path) => {
        entry_point!($path, config = &crate::BootloaderConfig::new_default());
    };
    ($path:path, config = $config:expr) => {
        #[link_section = ".bootloader-config"]
        pub static __BOOTLOADER_CONFIG: [u8; $crate::BootloaderConfig::SERIALIZED_LEN] = {
            // validate the type
            let config: &$crate::BootloaderConfig = $config;
            config.serialize()
        };

        #[export_name = "_start"]
        pub extern "C" fn __impl_start(boot_info: &'static mut $crate::info::BootInfo) -> ! {
            // validate the signature of the program entry point
            let f: fn(&'static mut $crate::info::BootInfo) -> ! = $path;

            // ensure that the config is used so that the linker keeps it
            $crate::__force_use(&__BOOTLOADER_CONFIG);

            f(boot_info)
        }
    };
}

#[doc(hidden)]
pub fn __force_use(slice: &[u8]) {
    let force_use = &slice as *const _ as usize;
    unsafe { asm!("add {0}, 0", in(reg) force_use, options(nomem, nostack)) };
}
