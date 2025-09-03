# Unreleased

# 0.9.33 – 2025-09-03

This release is compatible with Rust nightlies starting with `nightly-2025-08-28`.

- [Fix(v0.9): target-pointer-width field now expects an integer](https://github.com/rust-osdev/bootloader/pull/518)

# 0.9.32 – 2025-07-31

- [fix target-c-int-width](https://github.com/rust-osdev/bootloader/pull/512)

# 0.9.31 – 2025-03-22

- [remove #[no_mangle] from panic handler](https://github.com/rust-osdev/bootloader/pull/500)

# 0.9.30 – 2025-02-10

- [change rustc-abi in custom targets to x86-softfloat](https://github.com/rust-osdev/bootloader/pull/492)

# 0.9.29 – 2024-04-30

- Fix warnings ([#436](https://github.com/rust-osdev/bootloader/pull/436) and [#437](https://github.com/rust-osdev/bootloader/pull/437))

# 0.9.28 – 2024-02-28

- Fix warnings: [Remove unused paging imports](https://github.com/rust-osdev/bootloader/pull/430)

# 0.9.27 – 2024-02-16

- [Fix invalid mapping to zero page caused by off-by-one bug](https://github.com/rust-osdev/bootloader/pull/424)

# 0.9.26 – 2024-02-16

- [Fix map errors during kernel loading](https://github.com/rust-osdev/bootloader/pull/422)
    - Don't error if a kernel page is already mapped to the correct frame
    - Fix: unmap temp page again to enable multiple bss-like sections

# 0.9.25 – 2024-02-16

- [Fix data layout for custom targets for LLVM 18](https://github.com/rust-osdev/bootloader/pull/421)
  - Fixes build on latest Rust nightly

# 0.9.24 – 2024-01-28

- Fix data layout for `x86_64-bootloader` target ([#415](https://github.com/rust-osdev/bootloader/pull/415))
  - Fixes build on latest Rust nightly

# 0.9.23 – 2022-09-25

- Fix boot for machines that report memory regions at high physical addresses (see #259)
  - Allow allocating more than one level 4 entry ([#264](https://github.com/rust-osdev/bootloader/pull/264))

# 0.9.22 – 2022-03-01

- Remove the `asm` and `global_asm` features, which are now stable ([#227](https://github.com/rust-osdev/bootloader/pull/227))

# 0.9.21 – 2022-01-09

- Use new inline assembly syntax ([#204](https://github.com/rust-osdev/bootloader/pull/204))

# 0.9.20 – 2021-12-20

- Update x86_64 dependency to v0.14.7 to fix nightly breakage ([#208](https://github.com/rust-osdev/bootloader/pull/208))

# 0.9.19 – 2021-08-09

- Set `relocation-model: static` and `panic-strategy: abort` and `fix .intel_syntax` warnings [#185](https://github.com/rust-osdev/bootloader/pull/185)
  - Fixes warnings on the latest Rust nightlies.
  - This effectively changes the `relocation-model` and `panic-strategy`. Please report if you encounter any issues because of this.

# 0.9.18 – 2021-05-20

- Fix nightly regression by manually passing --gc-sections ([#168](https://github.com/rust-osdev/bootloader/pull/168))

# 0.9.17 – 2021-04-30

- Reduce the number of used unstable features of x86_64 crate (backport [#155](https://github.com/rust-osdev/bootloader/pull/140))

# 0.9.16 – 2021-03-07

Replace all remaining `lea`s with `mov` + `offset` ([#140](https://github.com/rust-osdev/bootloader/pull/140))

# 0.9.15 – 2021-03-07

- Fix linker errors on latest nightlies ([#139](https://github.com/rust-osdev/bootloader/pull/139))

# 0.9.14 – 2021-02-24

- Fix "panic message is not a string literal" warning ([#138](https://github.com/rust-osdev/bootloader/pull/138))

# 0.9.13 – 2021-02-24

(accidental release)

# 0.9.12 – 2021-02-02

- Fix build on latest nightly by updating x86_64 to v0.13.2 ([#135](https://github.com/rust-osdev/bootloader/pull/135))

# 0.9.11 – 2020-09-29

- Update `Cargo.lock` to fix nightly breakage ([#129](https://github.com/rust-osdev/bootloader/pull/129))

# 0.9.10 – 2020-09-24

- Update `x86_64` again to version 0.12.1 to fix `const fn`-related build errors on latest nightly

# 0.9.9 – 2020-09-20

- Run `cargo update` to fix build errors of `x86_64` on latest nightly

# 0.9.8 – 2020-07-17

- Enable rlibc dependency only with `binary` feature ([#126](https://github.com/rust-osdev/bootloader/pull/126))

# 0.9.7 – 2020-07-17

- Make bootloader buildable with `-Zbuild-std` ([#125](https://github.com/rust-osdev/bootloader/pull/125))

# 0.9.6 – 2020-07-16

- Change 1st stage int 13h addressing ([#123](https://github.com/rust-osdev/bootloader/pull/123))

# 0.9.5

- Fix warning by renaming `_improper_ctypes_check` functions ([#122](https://github.com/rust-osdev/bootloader/pull/122))

# 0.9.4

- Add recursive_idx for boot info ([#116](https://github.com/rust-osdev/bootloader/pull/116))
- Remove unused feature gates ([#118](https://github.com/rust-osdev/bootloader/pull/118))

# 0.9.3

- Update x86_64 dependency to version 0.11.0 ([#117](https://github.com/rust-osdev/bootloader/pull/117))

# 0.9.2

- **Nightly Breakage:** Use `llvm_asm!` instead of deprecated `asm!` ([#108](https://github.com/rust-osdev/bootloader/pull/108))

# 0.9.1

- SSE feature: remove inline assembly + don't set reserved bits ([#105](https://github.com/rust-osdev/bootloader/pull/105))

# 0.9.0

- **Breaking**: Identity-map complete vga region (0xa0000 to 0xc0000) ([#104](https://github.com/rust-osdev/bootloader/pull/104))

# 0.8.9

- Implement boot-info-address ([#101](https://github.com/rust-osdev/bootloader/pull/101))

# 0.8.8

- Add basic support for ELF thread local storage segments ([#96](https://github.com/rust-osdev/bootloader/pull/96))

# 0.8.7

- Fix docs.rs build (see commit 01671dbe449b85b3c0ea73c5796cc8f9661585ee)

# 0.8.6

- Objcopy replaces `.` chars with `_` chars ([#94](https://github.com/rust-osdev/bootloader/pull/94))

# 0.8.5

- Update x86_64 dependency ([#92](https://github.com/rust-osdev/bootloader/pull/92))

# 0.8.4

- Move architecture checks from build script into lib.rs ([#91](https://github.com/rust-osdev/bootloader/pull/91))

# 0.8.3

- Remove unnecessary `extern C` on panic handler to fix not-ffi-safe warning ([#85](https://github.com/rust-osdev/bootloader/pull/85))

# 0.8.2

- Change the way the kernel entry point is called to honor alignement ABI ([#81](https://github.com/rust-osdev/bootloader/pull/81))

# 0.8.1

- Add a Cargo Feature for Enabling SSE ([#77](https://github.com/rust-osdev/bootloader/pull/77))

# 0.8.0

- **Breaking**: Parse bootloader configuration from kernel's Cargo.toml ([#73](https://github.com/rust-osdev/bootloader/pull/73))
    - At least version 0.7.7 of `bootimage` is required now.
- Configurable kernel stack size, better non-x86_64 errors ([#72](https://github.com/rust-osdev/bootloader/pull/72))
- Dynamically map kernel stack, boot info, physical memory and recursive table ([#71](https://github.com/rust-osdev/bootloader/pull/71))

# 0.7.1

- Run cargo update (improves compile times because of trimmed down upstream dependencies)

# 0.7.0

- **Breaking**: Only include dependencies when `binary` feature is enabled ([#68](https://github.com/rust-osdev/bootloader/pull/68))
    - For manual builds, the `binary` feature must be enabled when building
    - For builds using `bootimage`, at least version 0.7.6 of `bootimage` is required now.

# 0.6.4

- Use volatile accesses in VGA code and make font dependency optional ([#67](https://github.com/rust-osdev/bootloader/pull/67))
  - Making the dependency optional should improve compile times when the VGA text mode is used.

# 0.6.3

- Update CI badge, use latest version of x86_64 crate and rustfmt ([#63](https://github.com/rust-osdev/bootloader/pull/63))

# 0.6.2

- Remove stabilized publish-lockfile feature ([#62](https://github.com/rust-osdev/bootloader/pull/62))

# 0.6.1

- Make the physical memory offset configurable through a `BOOTLOADER_PHYSICAL_MEMORY_OFFSET` environment variable ([#58](https://github.com/rust-osdev/bootloader/pull/58)).
- Use a stripped copy of the kernel binary (debug info removed) to reduce load times ([#59](https://github.com/rust-osdev/bootloader/pull/59)).

# 0.6.0

- **Breaking**: Don't set the `#[cfg(not(test))]` attribute for the entry point function in the `entry_point` macro
    - With custom test frameworks, it's possible to use the normal entry point also in test environments
    - To get the old behavior, you can add the `#[cfg(not(test))]` attribute to the `entry_point` invocation
- Additional assertions for the passed `KERNEL` executable
    - check that the executable exists (for better error messages)
    - check that the executable has a non-empty text section (an empty text section occurs when no entry point is set)

# 0.5.3

- Mention minimal required bootimage version in error message when `KERNEL` environment variable is not set.

# 0.5.2

- Remove redundant import that caused a warning

# 0.5.1

- Add a `package.metadata.bootloader.target` key to the Cargo.toml that can be used by tools such as `bootimage`.

# 0.5.0

- **Breaking**: Change the build system: Use a build script that expects a `KERNEL` environment variable instead of using a separate `builder` executable as before. See [#51](https://github.com/rust-osdev/bootloader/pull/51) and [#53](https://github.com/rust-osdev/bootloader/pull/53) for more information.
  - This makes the bootloader incompatible with versions `0.6.*` and earlier of the `bootimage` tool.
  - The bootloader also requires the `llvm-tools-preview` rustup component now.

# 0.4.0

## Breaking

- The level 4 page table is only recursively mapped if the `recursive_page_table` feature is enabled.
- Rename `BootInfo::p4_table_addr` to `BootInfo::recursive_page_table_addr` (only present if the cargo feature is enabled)
- Remove `From<PhysFrameRange>` implemenations for x86_64 `FrameRange`
  - This only works when the versions align, so it is not a good general solution.
- Remove unimplemented `BootInfo::package` field.
- Make `BootInfo` non-exhaustive so that we can add additional fields later.

## Other

- Add a `map_physical_memory` feature that maps the complete physical memory to the virtual address space at `BootInfo::physical_memory_offset`.
- Re-export `BootInfo` at the root.
