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
