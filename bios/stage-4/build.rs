use std::path::Path;

fn main() {
    let local_path = Path::new(env!("CARGO_MANIFEST_DIR"));
    println!(
        "cargo:rustc-link-arg-bins=--script={}",
        local_path.join("stage-4-link.ld").display()
    )
}
