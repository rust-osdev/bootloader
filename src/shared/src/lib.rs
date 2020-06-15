#![feature(abi_x86_interrupt)]
#![feature(const_fn)]
#![feature(llvm_asm, global_asm)]
#![no_std]

pub mod console;
pub mod dap;
pub mod utils;
#[macro_use]
pub mod macros;
pub mod structures;
pub mod instructions;