//! This library part of the bootloader allows kernels to retrieve information from the bootloader.
//!
//! To combine your kernel with the bootloader crate you need a tool such
//! as [`bootimage`](https://github.com/rust-osdev/bootimage). See the
//! [_Writing an OS in Rust_](https://os.phil-opp.com/minimal-rust-kernel/#creating-a-bootimage)
//! blog for an explanation.

#![cfg_attr(not(feature = "builder"), no_std)]
#![feature(min_const_generics)]
#![feature(slice_fill)]
#![feature(asm)]
#![feature(unsafe_block_in_unsafe_fn)]
#![feature(maybe_uninit_slice_assume_init)]
#![feature(maybe_uninit_extra)]
#![deny(unsafe_op_in_unsafe_fn)]
//#![warn(missing_docs)]

pub use crate::bootinfo::BootInfo;

#[cfg(feature = "bios_bin")]
use x86_64::{
    structures::paging::{frame::PhysFrameRange, PhysFrame},
    PhysAddr,
};

pub mod bootinfo;

pub mod boot_info;
pub mod memory_map;

#[cfg(feature = "binary")]
pub mod binary;

#[cfg(feature = "builder")]
pub mod disk_image;

#[cfg(feature = "bios_bin")]
pub mod level4_entries;

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

#[cfg(feature = "bios_bin")]
pub fn phys_frame_range(range: bootinfo::FrameRange) -> PhysFrameRange {
    PhysFrameRange {
        start: PhysFrame::from_start_address(PhysAddr::new(range.start_addr())).unwrap(),
        end: PhysFrame::from_start_address(PhysAddr::new(range.end_addr())).unwrap(),
    }
}

#[cfg(feature = "bios_bin")]
pub fn frame_range(range: PhysFrameRange) -> bootinfo::FrameRange {
    bootinfo::FrameRange::new(
        range.start.start_address().as_u64(),
        range.end.start_address().as_u64(),
    )
}
