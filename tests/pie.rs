use std::process::Command;

#[test]
fn basic_boot() { run_test_binary("basic_boot"); }

#[test]
fn should_panic() { run_test_binary("should_panic"); }

#[test]
fn check_boot_info() { run_test_binary("check_boot_info"); }

#[test]
fn global_variable() { run_test_binary("global_variable"); }

fn run_test_binary(bin_name: &str) {
    let mut cmd = Command::new(env!("CARGO"));
    cmd.current_dir("tests/test_kernels/pie");
    cmd.arg("run");
    cmd.arg("--bin").arg(bin_name);
    cmd.arg("--target").arg("x86_64-pie.json");
    cmd.arg("-Zbuild-std=core");
    cmd.arg("-Zbuild-std-features=compiler-builtins-mem");
    assert!(cmd.status().unwrap().success());
}
