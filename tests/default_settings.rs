use bootloader_test_runner::define_test;
const BASIC_BOOT_KERNEL: &str = env!("CARGO_BIN_FILE_TEST_KERNEL_DEFAULT_SETTINGS_basic_boot");
const SHOULD_PANIC_KERNEL: &str = env!("CARGO_BIN_FILE_TEST_KERNEL_DEFAULT_SETTINGS_should_panic");
const CHECK_BOOT_INFO_KERNEL: &str = env!("CARGO_BIN_FILE_TEST_KERNEL_DEFAULT_SETTINGS_check_boot_info");

define_test!(basic_boot, BASIC_BOOT_KERNEL);
define_test!(should_panic, SHOULD_PANIC_KERNEL);
define_test!(check_boot_info, CHECK_BOOT_INFO_KERNEL);
define_test!(disable_default_ramdisk_macro_test, BASIC_BOOT_KERNEL, without_ramdisk_tests);
