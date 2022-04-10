#[cfg(not(feature = "binary"))]
fn main() {}

#[cfg(feature = "binary")]
fn main() {
    binary::main();
}

#[cfg(feature = "binary")]
mod binary {
    use quote::quote;
    use std::convert::TryInto;

    pub fn main() {
        use llvm_tools_build as llvm_tools;
        use std::{
            env,
            fs::{self, File},
            io::Write,
            path::{Path, PathBuf},
            process::{self, Command},
        };
        use toml::Value;

        let target = env::var("TARGET").expect("TARGET not set");
        let (firmware, expected_target) = if cfg!(feature = "uefi_bin") {
            ("UEFI", "x86_64-unknown-uefi")
        } else if cfg!(feature = "bios_bin") {
            ("BIOS", "x86_64-bootloader")
        } else {
            panic!(
                "Either the `uefi_bin` or `bios_bin` feature must be enabled when \
            the `binary` feature is enabled"
            );
        };
        if Path::new(&target)
            .file_stem()
            .expect("target has no file stem")
            != expected_target
        {
            panic!(
                "The {} bootloader must be compiled for the `{}` target.",
                firmware, expected_target,
            );
        }

        let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
        let kernel = PathBuf::from(match env::var("KERNEL") {
            Ok(kernel) => kernel,
            Err(_) => {
                eprintln!(
                    "The KERNEL environment variable must be set for building the bootloader.\n\n\
                 Please use the `cargo builder` command for building."
                );
                process::exit(1);
            }
        });
        let kernel_file_name = kernel
            .file_name()
            .expect("KERNEL has no valid file name")
            .to_str()
            .expect("kernel file name not valid utf8");

        // check that the kernel file exists
        assert!(
            kernel.exists(),
            "KERNEL does not exist: {}",
            kernel.display()
        );

        // get access to llvm tools shipped in the llvm-tools-preview rustup component
        let llvm_tools = match llvm_tools::LlvmTools::new() {
            Ok(tools) => tools,
            Err(llvm_tools::Error::NotFound) => {
                eprintln!("Error: llvm-tools not found");
                eprintln!("Maybe the rustup component `llvm-tools-preview` is missing?");
                eprintln!("  Install it through: `rustup component add llvm-tools-preview`");
                process::exit(1);
            }
            Err(err) => {
                eprintln!("Failed to retrieve llvm-tools component: {:?}", err);
                process::exit(1);
            }
        };

        // check that kernel executable has code in it
        let llvm_size = llvm_tools
            .tool(&llvm_tools::exe("llvm-size"))
            .expect("llvm-size not found in llvm-tools");
        let mut cmd = Command::new(llvm_size);
        cmd.arg(&kernel);
        let output = cmd.output().expect("failed to run llvm-size");
        let output_str = String::from_utf8_lossy(&output.stdout);
        let second_line_opt = output_str.lines().skip(1).next();
        let second_line = second_line_opt.expect(&format!(
            "unexpected llvm-size line output:\n{}",
            output_str
        ));
        let text_size_opt = second_line.split_ascii_whitespace().next();
        let text_size =
            text_size_opt.expect(&format!("unexpected llvm-size output:\n{}", output_str));
        if text_size == "0" {
            panic!("Kernel executable has an empty text section. Perhaps the entry point was set incorrectly?\n\n\
            Kernel executable at `{}`\n", kernel.display());
        }

        // strip debug symbols from kernel for faster loading
        let stripped_kernel_file_name = format!("kernel_stripped-{}", kernel_file_name);
        let stripped_kernel = out_dir.join(&stripped_kernel_file_name);
        let objcopy = llvm_tools
            .tool(&llvm_tools::exe("llvm-objcopy"))
            .expect("llvm-objcopy not found in llvm-tools");
        let mut cmd = Command::new(&objcopy);
        cmd.arg("--strip-debug");
        cmd.arg(&kernel);
        cmd.arg(&stripped_kernel);
        let exit_status = cmd
            .status()
            .expect("failed to run objcopy to strip debug symbols");
        if !exit_status.success() {
            eprintln!("Error: Stripping debug symbols failed");
            process::exit(1);
        }

        if cfg!(feature = "uefi_bin") {
            // write file for including kernel in binary
            let file_path = out_dir.join("kernel_info.rs");
            let mut file = File::create(file_path).expect("failed to create kernel_info.rs");
            let kernel_size = fs::metadata(&stripped_kernel)
                .expect("Failed to read file metadata of stripped kernel")
                .len();
            file.write_all(
                format!(
                    "const KERNEL_SIZE: usize = {}; const KERNEL_BYTES: [u8; KERNEL_SIZE] = *include_bytes!(r\"{}\");",
                    kernel_size,
                    stripped_kernel.display(),
                )
                .as_bytes(),
            )
            .expect("write to kernel_info.rs failed");
        }

        if cfg!(feature = "bios_bin") {
            // wrap the kernel executable as binary in a new ELF file
            let stripped_kernel_file_name_replaced = stripped_kernel_file_name
                .replace('-', "_")
                .replace('.', "_");
            let kernel_bin = out_dir.join(format!("kernel_bin-{}.o", kernel_file_name));
            let kernel_archive = out_dir.join(format!("libkernel_bin-{}.a", kernel_file_name));
            let mut cmd = Command::new(&objcopy);
            cmd.arg("-I").arg("binary");
            cmd.arg("-O").arg("elf64-x86-64");
            cmd.arg("--binary-architecture=i386:x86-64");
            cmd.arg("--rename-section").arg(".data=.kernel");
            cmd.arg("--redefine-sym").arg(format!(
                "_binary_{}_start=_kernel_start_addr",
                stripped_kernel_file_name_replaced
            ));
            cmd.arg("--redefine-sym").arg(format!(
                "_binary_{}_end=_kernel_end_addr",
                stripped_kernel_file_name_replaced
            ));
            cmd.arg("--redefine-sym").arg(format!(
                "_binary_{}_size=_kernel_size",
                stripped_kernel_file_name_replaced
            ));
            cmd.current_dir(&out_dir);
            cmd.arg(&stripped_kernel_file_name);
            cmd.arg(&kernel_bin);
            let exit_status = cmd.status().expect("failed to run objcopy");
            if !exit_status.success() {
                eprintln!("Error: Running objcopy failed");
                process::exit(1);
            }

            // create an archive for linking
            let ar = llvm_tools
                .tool(&llvm_tools::exe("llvm-ar"))
                .unwrap_or_else(|| {
                    eprintln!("Failed to retrieve llvm-ar component");
                    eprint!("This component is available since nightly-2019-03-29,");
                    eprintln!("so try updating your toolchain if you're using an older nightly");
                    process::exit(1);
                });
            let mut cmd = Command::new(ar);
            cmd.arg("crs");
            cmd.arg(&kernel_archive);
            cmd.arg(&kernel_bin);
            let exit_status = cmd.status().expect("failed to run ar");
            if !exit_status.success() {
                eprintln!("Error: Running ar failed");
                process::exit(1);
            }

            // pass link arguments to rustc
            println!("cargo:rustc-link-search=native={}", out_dir.display());
            println!(
                "cargo:rustc-link-lib=static=kernel_bin-{}",
                kernel_file_name
            );
        }

        // Parse configuration from the kernel's Cargo.toml
        let mut config = None;
        let config_stream = match env::var("KERNEL_MANIFEST") {
            Err(env::VarError::NotPresent) => {
                panic!("The KERNEL_MANIFEST environment variable must be set for building the bootloader.\n\n\
                 Please use `cargo builder` for building.");
            }
            Err(env::VarError::NotUnicode(_)) => {
                panic!("The KERNEL_MANIFEST environment variable contains invalid unicode")
            }
            Ok(path)
                if Path::new(&path).file_name().and_then(|s| s.to_str()) != Some("Cargo.toml") =>
            {
                let err = format!(
                    "The given `--kernel-manifest` path `{}` does not \
                    point to a `Cargo.toml`",
                    path,
                );
                quote! { compile_error!(#err) }
            }
            Ok(path) if !Path::new(&path).exists() => {
                let err = format!(
                    "The given `--kernel-manifest` path `{}` does not exist.",
                    path
                );
                quote! {
                    compile_error!(#err)
                }
            }
            Ok(path) => {
                println!("cargo:rerun-if-changed={}", path);

                let contents = fs::read_to_string(&path).expect(&format!(
                    "failed to read kernel manifest file (path: {})",
                    path
                ));

                let manifest = contents
                    .parse::<Value>()
                    .expect("failed to parse kernel's Cargo.toml");

                if manifest
                    .get("dependencies")
                    .and_then(|d| d.get("bootloader"))
                    .or_else(|| {
                        manifest
                            .get("target")
                            .and_then(|table| table.get(r#"cfg(target_arch = "x86_64")"#))
                            .and_then(|table| table.get("dependencies"))
                            .and_then(|table| table.get("bootloader"))
                    })
                    .is_some()
                {
                    // it seems to be the correct Cargo.toml
                    let config_table = manifest
                        .get("package")
                        .and_then(|table| table.get("metadata"))
                        .and_then(|table| table.get("bootloader"))
                        .cloned()
                        .unwrap_or_else(|| toml::Value::Table(toml::map::Map::new()));

                    let result = config_table.try_into::<ParsedConfig>();
                    match result {
                        Ok(p_config) => {
                            let stream = quote! { #p_config };
                            config = Some(p_config);
                            stream
                        }
                        Err(err) => {
                            let err = format!(
                                "failed to parse bootloader config in {}:\n\n{}",
                                path,
                                err.to_string()
                            );
                            quote! {
                                compile_error!(#err)
                            }
                        }
                    }
                } else {
                    let err = format!(
                        "no bootloader dependency in {}\n\n  The \
                    `--kernel-manifest` path should point to the `Cargo.toml` \
                    of the kernel.",
                        path
                    );
                    quote! {
                        compile_error!(#err)
                    }
                }
            }
        };
        let config = config;

        // Write config to file
        let file_path = out_dir.join("bootloader_config.rs");
        let mut file = File::create(file_path).expect("failed to create config file");
        file.write_all(
            quote::quote! {
                /// Module containing the user-supplied configuration.
                /// Public so that `bin/uefi.rs` can read framebuffer configuration.
                pub mod parsed_config {
                    use crate::config::Config;
                    /// The parsed configuration given by the user.
                    pub const CONFIG: Config = #config_stream;
                }
            }
            .to_string()
            .as_bytes(),
        )
        .expect("writing config failed");

        // Write VESA framebuffer configuration
        let file_path = out_dir.join("vesa_config.s");
        let mut file = File::create(file_path).expect("failed to create vesa config file");
        file.write_fmt(format_args!(
            "vesa_minx: .2byte {}\n\
            vesa_miny: .2byte {}",
            config
                .as_ref()
                .map(|c| c.minimum_framebuffer_width)
                .flatten()
                .unwrap_or(640),
            config
                .as_ref()
                .map(|c| c.minimum_framebuffer_height)
                .flatten()
                .unwrap_or(480)
        ))
        .expect("writing config failed");

        println!("cargo:rerun-if-env-changed=KERNEL");
        println!("cargo:rerun-if-env-changed=KERNEL_MANIFEST");
        println!("cargo:rerun-if-changed={}", kernel.display());
        println!("cargo:rerun-if-changed=build.rs");
    }

    fn val_true() -> bool {
        true
    }

    /// Must be always identical with the struct in `src/config.rs`
    ///
    /// This copy is needed because we can't derive Deserialize in the `src/config.rs`
    /// module itself, since cargo currently unifies dependencies (the `toml` crate enables
    /// serde's standard feature). Also, it allows to separate the parsing special cases
    /// such as `AlignedAddress` more cleanly.
    #[derive(Debug, serde::Deserialize)]
    #[serde(rename_all = "kebab-case", deny_unknown_fields)]
    struct ParsedConfig {
        #[serde(default)]
        pub map_physical_memory: bool,
        #[serde(default)]
        pub map_page_table_recursively: bool,
        #[serde(default = "val_true")]
        pub map_framebuffer: bool,
        #[serde(default)]
        pub aslr: bool,
        pub kernel_stack_size: Option<AlignedAddress>,
        pub physical_memory_offset: Option<AlignedAddress>,
        pub recursive_index: Option<u16>,
        pub kernel_stack_address: Option<AlignedAddress>,
        pub boot_info_address: Option<AlignedAddress>,
        pub framebuffer_address: Option<AlignedAddress>,
        pub minimum_framebuffer_height: Option<usize>,
        pub minimum_framebuffer_width: Option<usize>,
        pub dynamic_range_start: Option<AlignedAddress>,
        pub dynamic_range_end: Option<AlignedAddress>,
    }

    /// Convert to tokens suitable for initializing the `Config` struct.
    impl quote::ToTokens for ParsedConfig {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            fn optional(value: Option<impl quote::ToTokens>) -> proc_macro2::TokenStream {
                value.map(|v| quote!(Some(#v))).unwrap_or(quote!(None))
            }

            let map_physical_memory = self.map_physical_memory;
            let map_page_table_recursively = self.map_page_table_recursively;
            let map_framebuffer = self.map_framebuffer;
            let aslr = self.aslr;
            let kernel_stack_size = optional(self.kernel_stack_size);
            let physical_memory_offset = optional(self.physical_memory_offset);
            let recursive_index = optional(self.recursive_index);
            let kernel_stack_address = optional(self.kernel_stack_address);
            let boot_info_address = optional(self.boot_info_address);
            let framebuffer_address = optional(self.framebuffer_address);
            let minimum_framebuffer_height = optional(self.minimum_framebuffer_height);
            let minimum_framebuffer_width = optional(self.minimum_framebuffer_width);
            let dynamic_range_start = optional(self.dynamic_range_start);
            let dynamic_range_end = optional(self.dynamic_range_end);

            tokens.extend(quote! { Config {
                map_physical_memory: #map_physical_memory,
                map_page_table_recursively: #map_page_table_recursively,
                map_framebuffer: #map_framebuffer,
                aslr: #aslr,
                kernel_stack_size: #kernel_stack_size,
                physical_memory_offset: #physical_memory_offset,
                recursive_index: #recursive_index,
                kernel_stack_address: #kernel_stack_address,
                boot_info_address: #boot_info_address,
                framebuffer_address: #framebuffer_address,
                minimum_framebuffer_height: #minimum_framebuffer_height,
                minimum_framebuffer_width: #minimum_framebuffer_width,
                dynamic_range_start: #dynamic_range_start,
                dynamic_range_end: #dynamic_range_end,
            }});
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct AlignedAddress(u64);

    impl quote::ToTokens for AlignedAddress {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            self.0.to_tokens(tokens);
        }
    }

    impl<'de> serde::Deserialize<'de> for AlignedAddress {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_str(AlignedAddressVisitor)
        }
    }

    /// Helper struct for implementing the `optional_version_deserialize` function.
    struct AlignedAddressVisitor;

    impl serde::de::Visitor<'_> for AlignedAddressVisitor {
        type Value = AlignedAddress;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "a page-aligned memory address, either as integer or as decimal or hexadecimal \
                string (e.g. \"0xffff0000\"); large addresses must be given as string because \
                TOML does not support unsigned 64-bit integers"
            )
        }

        fn visit_u64<E>(self, num: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if num % 0x1000 == 0 {
                Ok(AlignedAddress(num))
            } else {
                Err(serde::de::Error::custom(format!(
                    "address {:#x} is not page aligned",
                    num
                )))
            }
        }

        fn visit_i64<E>(self, num: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let unsigned: u64 = num
                .try_into()
                .map_err(|_| serde::de::Error::custom(format!("address {} is negative", num)))?;
            self.visit_u64(unsigned)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // ignore any `_` (used for digit grouping)
            let value = &value.replace('_', "");

            let num = if value.starts_with("0x") {
                u64::from_str_radix(&value[2..], 16)
            } else {
                u64::from_str_radix(&value, 10)
            }
            .map_err(|_err| {
                serde::de::Error::custom(format!(
                    "string \"{}\" is not a valid memory address",
                    value
                ))
            })?;

            self.visit_u64(num)
        }
    }
}
