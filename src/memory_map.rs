#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MemoryRegion {
    pub start: u64,
    pub end: u64,
    pub kind: MemoryRegionKind,
}

impl MemoryRegion {
    pub const fn empty() -> Self {
        MemoryRegion {
            start: 0,
            end: 0,
            kind: MemoryRegionKind::Bootloader,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum MemoryRegionKind {
    /// Unused conventional memory, can be used by the kernel.
    Usable,
    /// Memory mappings created by the bootloader, including the kernel and boot info mappings.
    ///
    /// This memory should _not_ be used by the kernel.
    Bootloader,
    /// An unknown memory region reported by the UEFI firmware.
    ///
    /// This should only be used if the UEFI memory type is known as usable.
    UnknownUefi(u32),
    /// An unknown memory region reported by the BIOS firmware.
    ///
    /// This should only be used if the BIOS memory type is known as usable.
    UnknownBios(u32),
}
