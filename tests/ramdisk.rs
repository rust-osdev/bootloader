use bootloader_test_runner::define_test;
static RAMDISK_PATH: &str = "tests/ramdisk.txt";
static BASIC_BOOT_KERNEL: &str = env!("CARGO_BIN_FILE_TEST_KERNEL_RAMDISK_basic_boot");
static RAMDISK_KERNEL: &str = env!("CARGO_BIN_FILE_TEST_KERNEL_RAMDISK_ramdisk");
define_test!(basic_boot, BASIC_BOOT_KERNEL, RAMDISK_PATH);
define_test!(ramdisk, RAMDISK_KERNEL, RAMDISK_PATH);
