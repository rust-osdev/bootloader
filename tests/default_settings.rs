use std::process::Command;

#[test]
fn basic_boot() {
    run_test_binary("basic_boot");
}

#[test]
fn should_panic() {
    run_test_binary("should_panic");
}

fn run_test_binary(bin_name: &str) {
    let mut cmd = Command::new(env!("CARGO"));
    cmd.current_dir("tests/test_kernels/default_settings");
    cmd.arg("run");
    cmd.arg("--bin").arg(bin_name);
    cmd.arg("--target").arg(" x86_64-example-kernel.json");
    cmd.arg("-Zbuild-std=core");
    assert!(cmd.status().unwrap().success());
}
