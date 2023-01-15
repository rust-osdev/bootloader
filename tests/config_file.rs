use std::path::Path;

use bootloader_test_runner::run_test_kernel;

#[test]
fn basic_boot() {
    run_test_kernel(
        env!("CARGO_BIN_FILE_TEST_KERNEL_CONFIG_FILE_basic_boot"),
        None,
        Some(Path::new("tests/config_files/full_config.json")),
    );
}

#[test]
fn basic_boot_broken_config_file() {
    run_test_kernel(
        env!("CARGO_BIN_FILE_TEST_KERNEL_CONFIG_FILE_basic_boot_broken_config_file"),
        None,
        Some(Path::new("tests/config_files/broken_config.json")),
    );
}
