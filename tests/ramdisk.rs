use std::path::Path;

use bootloader_test_runner::run_test_kernel_with_ramdisk;
static RAMDISK_PATH: &str = "tests/ramdisk.txt";

#[test]
fn basic_boot() {
    run_test_kernel_with_ramdisk(
        env!("CARGO_BIN_FILE_TEST_KERNEL_RAMDISK_basic_boot"),
        Some(Path::new(RAMDISK_PATH)),
    );
}

#[test]
fn check_ramdisk() {
    run_test_kernel_with_ramdisk(
        env!("CARGO_BIN_FILE_TEST_KERNEL_RAMDISK_ramdisk"),
        Some(Path::new(RAMDISK_PATH)),
    );
}

#[test]
fn memory_map() {
    run_test_kernel_with_ramdisk(
        env!("CARGO_BIN_FILE_TEST_KERNEL_RAMDISK_memory_map"),
        Some(Path::new(RAMDISK_PATH)),
    );
}
