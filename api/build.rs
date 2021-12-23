use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("concat.rs");

    let combinations = [
        (1, 8),
        (1, 9),
        (2, 1),
        (2, 2),
        (4, 3),
        (16, 7),
        (23, 8),
        (31, 9),
        (40, 9),
        (49, 9),
        (58, 10),
        (68, 10),
        (78, 9),
        (87, 9),
    ];

    let mut code = String::new();
    for (i, j) in combinations {
        code += &format!(
            "pub const fn concat_{i}_{j}(a: [u8; {i}], b: [u8; {j}]) -> [u8; {i} + {j}] {{
                    [{a}, {b}]
                }}",
            i = i,
            j = j,
            a = (0..i)
                .map(|idx| format!("a[{}]", idx))
                .collect::<Vec<_>>()
                .join(","),
            b = (0..j)
                .map(|idx| format!("b[{}]", idx))
                .collect::<Vec<_>>()
                .join(","),
        );
    }

    fs::write(&dest_path, code).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}
