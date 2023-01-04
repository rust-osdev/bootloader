use bootloader_test_runner::define_test;
const CHECK_BOOT_INFO_KERNEL: &str = env!("CARGO_BIN_FILE_TEST_KERNEL_MAP_PHYS_MEM_check_boot_info");
const ACCESS_PHYS_MEM_KERNEL: &str = env!("CARGO_BIN_FILE_TEST_KERNEL_MAP_PHYS_MEM_access_phys_mem");

define_test!(check_boot_info, CHECK_BOOT_INFO_KERNEL);
define_test!(access_phys_mem, ACCESS_PHYS_MEM_KERNEL);
