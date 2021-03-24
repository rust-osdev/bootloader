//! This library part of the bootloader allows kernels to retrieve information from the bootloader.
//!
//! To combine your kernel with the bootloader crate you need a tool such
//! as [`bootimage`](https://github.com/rust-osdev/bootimage). See the
//! [_Writing an OS in Rust_](https://os.phil-opp.com/minimal-rust-kernel/#creating-a-bootimage)
//! blog for an explanation.

#![cfg_attr(not(feature = "builder"), no_std)]
#![feature(asm)]
#![feature(maybe_uninit_extra)]
#![feature(maybe_uninit_slice)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

pub use crate::boot_info::BootInfo;

/// Configuration options for the bootloader.
pub mod config;

/// Contains the boot information struct sent by the bootloader to the kernel on startup.
pub mod boot_info;
/// Provides a memory region type that is used in the memory map of the boot info structure.
pub mod memory_region;

/// Contains the actual bootloader implementation ("bootloader as a binary").
///
/// Useful for reusing part of the bootloader implementation for other crates.
///
/// Only available when the `binary` feature is enabled.
#[cfg(feature = "binary")]
pub mod binary;

/// Provides a function to turn a bootloader executable into a disk image.
///
/// Used by the `builder` binary. Only available when the `builder` feature is enabled.
#[cfg(feature = "builder")]
pub mod disk_image;

#[cfg(target_arch = "x86")]
compile_error!(
    "This crate currently does not support 32-bit protected mode. \
         See https://github.com/rust-osdev/bootloader/issues/70 for more information."
);

#[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
compile_error!("This crate only supports the x86_64 architecture.");

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
        #[export_name = "_start"]
        pub extern "C" fn __impl_start(boot_info: &'static mut $crate::boot_info::BootInfo) -> ! {
            // validate the signature of the program entry point
            let f: fn(&'static mut $crate::boot_info::BootInfo) -> ! = $path;

            f(boot_info)
        }
    };
}
