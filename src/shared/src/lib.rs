#![feature(abi_x86_interrupt)]
#![feature(const_fn)]
#![feature(asm, global_asm)]
#![no_std]

pub mod console;
pub mod dap;
#[macro_use]
pub mod macros;
pub mod structures;
pub mod instructions;

pub mod memory_operations;