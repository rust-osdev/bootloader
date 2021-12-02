use std::{env, fs, path::Path};

// #![feature(split_array)]

// use config::{BootloaderConfig, Version};

// fn main() {
//     let BootloaderConfig {
//         version:
//             Version {
//                 version_major,
//                 version_minor,
//                 version_patch,
//                 pre_release,
//             },
//         mappings,
//         kernel_stack_size,
//         frame_buffer,
//     } = BootloaderConfig::default();

//     let version_major = version_major.to_le_bytes();
//     let version_minor = version_minor.to_le_bytes();
//     let version_patch = version_patch.to_le_bytes();
//     let pre_release = [pre_release as u8];

//     let version_fields = [
//         ("version_major", version_major.len()),
//         ("version_minor", version_minor.len()),
//         ("version_patch", version_patch.len()),
//         ("pre_release", pre_release.len()),
//     ];
//     let version_len = version_fields.iter().map(|(_, l)| l).sum::<usize>();

//     let kernel_stack_size = kernel_stack_size.to_le_bytes();
//     let fields = [
//         ("Self::UUID", BootloaderConfig::UUID.len()),
//         ("version", version_len),
//         ("kernel_stack_size", kernel_stack_size.len()),
//         ("mappings", mappings_len),
//         ("frame_buffer", frame_buffer_len),
//     ];

//     let total_len = fields.iter().map(|(_, l)| l).sum::<usize>();

//     let x = format!(
//         "
//         impl Version {{
//             pub SERIALIZED_LEN: usize = {};
//             pub const fn serialize(&self) -> [u8; Self::SERIALIZED_LEN] {{
//                 {}
//             }}
//         }}

//         impl BootloaderConfig {{
//             pub SERIALIZED_LEN: usize = {};
//             pub const fn serialize(&self) -> [u8; Self::SERIALIZED_LEN] {{
//                 []
//             }}
//         }}
//     ",
//         version_len, total_len
//     );

//     panic!("{}", x);
// }

// #[path = "src/config.rs"]
// mod config;

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
