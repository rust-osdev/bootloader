use bootloader_test_runner::run_test_kernel_internal;

use bootloader::BootConfig;

#[test]
fn basic_boot() {
    let config: BootConfig = Default::default();
    run_test_kernel_internal(
        env!("CARGO_BIN_FILE_TEST_KERNEL_CONFIG_FILE_basic_boot"),
        None,
        Some(&config),
    );
}

#[test]
fn custom_options_boot() {
    let config = BootConfig {
        frame_buffer: Default::default(),
        log_level: Default::default(),
        frame_buffer_logger_status: false,
        serial_logger_status: true,
    };
    run_test_kernel_internal(
        env!("CARGO_BIN_FILE_TEST_KERNEL_CONFIG_FILE_basic_boot_broken_config_file"),
        None,
        Some(&config),
    );
}
