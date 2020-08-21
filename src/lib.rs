//! This library part of the bootloader allows kernels to retrieve information from the bootloader.
//!
//! To combine your kernel with the bootloader crate you need a tool such
//! as [`bootimage`](https://github.com/rust-osdev/bootimage). See the
//! [_Writing an OS in Rust_](https://os.phil-opp.com/minimal-rust-kernel/#creating-a-bootimage)
//! blog for an explanation.

#![no_std]
#![feature(min_const_generics)]
#![feature(slice_fill)]
#![feature(asm)]
#![feature(unsafe_block_in_unsafe_fn)]
#![deny(unsafe_op_in_unsafe_fn)]
//#![warn(missing_docs)]

pub use crate::bootinfo::BootInfo;

use core::panic::PanicInfo;
#[cfg(feature = "uefi_bin")]
pub use logger::{FrameBufferInfo, PixelFormat};
#[cfg(feature = "uefi_bin")]
use x86_64::{
    structures::paging::{FrameAllocator, MapperAllSizes, Size4KiB},
    VirtAddr,
};

pub mod bootinfo;

pub mod boot_info_uefi;
pub mod memory_map;

#[cfg(feature = "uefi_bin")]
mod load_kernel;
#[cfg(feature = "uefi_bin")]
pub mod logger;

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
        pub extern "C" fn __impl_start(boot_info: &'static $crate::bootinfo::BootInfo) -> ! {
            // validate the signature of the program entry point
            let f: fn(&'static $crate::bootinfo::BootInfo) -> ! = $path;

            f(boot_info)
        }
    };
}

#[cfg(feature = "uefi_bin")]
pub fn init_logger(framebuffer: &'static mut [u8], info: FrameBufferInfo) {
    let logger = logger::LOGGER.get_or_init(move || logger::LockedLogger::new(framebuffer, info));
    log::set_logger(logger).expect("logger already set");
    log::set_max_level(log::LevelFilter::Trace);
}

#[cfg(feature = "uefi_bin")]
pub fn load_kernel(
    kernel: &'static [u8],
    page_table: &mut impl MapperAllSizes,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> VirtAddr {
    load_kernel::load_kernel(kernel, page_table, frame_allocator).expect("Failed to parse kernel")
}
