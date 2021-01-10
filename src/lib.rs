//! This library part of the bootloader allows kernels to retrieve information from the bootloader.
//!
//! To combine your kernel with the bootloader crate you need a tool such
//! as [`bootimage`](https://github.com/rust-osdev/bootimage). See the
//! [_Writing an OS in Rust_](https://os.phil-opp.com/minimal-rust-kernel/#creating-a-bootimage)
//! blog for an explanation.

#![cfg_attr(not(feature = "builder"), no_std)]
#![feature(asm)]
#![feature(unsafe_block_in_unsafe_fn)]
#![feature(maybe_uninit_extra)]
#![feature(maybe_uninit_slice)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

pub use crate::boot_info::BootInfo;

pub mod config;

pub mod boot_info;
pub mod memory_region;

#[cfg(feature = "binary")]
pub mod binary;

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
