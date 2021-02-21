use std::process::Command;

#[test]
fn check_boot_info() {
    run_test_binary("check_boot_info");
}

#[test]
fn access_phys_mem() {
    run_test_binary("access_phys_mem");
}

fn run_test_binary(bin_name: &str) {
    let mut cmd = Command::new(env!("CARGO"));
    cmd.current_dir("tests/test_kernels/map_phys_mem");
    cmd.arg("run");
    cmd.arg("--bin").arg(bin_name);
    cmd.arg("--target").arg("x86_64-map_phys_mem.json");
    cmd.arg("-Zbuild-std=core");
    cmd.arg("-Zbuild-std-features=compiler-builtins-mem");
    assert!(cmd.status().unwrap().success());
}
