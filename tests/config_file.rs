use bootloader_test_runner::run_test_kernel_internal;

use bootloader::BootConfig;

#[test]
fn default_config() {
    run_test_kernel_internal(
        env!("CARGO_BIN_FILE_TEST_KERNEL_CONFIG_FILE_no_config"),
        None,
        None,
    );
}

#[test]
fn custom_boot_config() {
    let config = BootConfig {
        frame_buffer: Default::default(),
        log_level: Default::default(),
        frame_buffer_logging: false,
        serial_logging: true,
        _test_sentinel: 0xb001b001b001,
    };
    run_test_kernel_internal(
        env!("CARGO_BIN_FILE_TEST_KERNEL_CONFIG_FILE_custom_config"),
        None,
        Some(&config),
    );
}
