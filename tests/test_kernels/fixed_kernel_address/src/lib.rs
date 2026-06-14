#![no_std]

use bootloader_api::{BootloaderConfig, config::Mapping};
use uart_16550::backend::PioBackend;
use uart_16550::{Config, Uart16550Tty};

pub const KERNEL_ADDR: u64 = 0x1987_6543_0000;

pub const BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.kernel_base = Mapping::FixedAddress(KERNEL_ADDR);
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
