fn main() {
    println!("cargo:rerun-if-changed=kernel.bin");
    println!("cargo:rerun-if-changed=linker.ld");
}
