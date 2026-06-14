#![no_std]

use bootloader_api::BootloaderConfig;
use uart_16550::backend::PioBackend;
use uart_16550::{Config, Uart16550Tty};

pub const BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.dynamic_range_start = Some(0xffff_8000_0000_0000);
    config
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    use x86_64::instructions::{nop, port::Port};

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }

    loop {
        nop();
    }
}

pub fn serial() -> Uart16550Tty<PioBackend> {
    unsafe { Uart16550Tty::new_port(0x3F8, Config::default()) }
        .expect("should initialize serial device from valid config and valid port")
}
