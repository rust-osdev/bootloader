use bootloader_test_runner::define_test;
const BASIC_BOOT_KERNEL: &str = env!("CARGO_BIN_FILE_TEST_KERNEL_PIE_basic_boot");
const SHOULD_PANIC_KERNEL: &str = env!("CARGO_BIN_FILE_TEST_KERNEL_PIE_should_panic");
const CHECK_BOOT_INFO_KERNEL: &str = env!("CARGO_BIN_FILE_TEST_KERNEL_PIE_check_boot_info");
const GLOBAL_VARIABLE_KERNEL: &str = env!("CARGO_BIN_FILE_TEST_KERNEL_PIE_global_variable");
define_test!(basic_boot, BASIC_BOOT_KERNEL);
define_test!(should_panic, SHOULD_PANIC_KERNEL);
define_test!(check_boot_info, CHECK_BOOT_INFO_KERNEL);
define_test!(global_variable, GLOBAL_VARIABLE_KERNEL);
