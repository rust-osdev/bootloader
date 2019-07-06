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
