use crate::concat::*;

/// Allows configuring the bootloader behavior.
///
/// TODO: describe use together with `entry_point` macro
/// TODO: example
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct BootloaderConfig {
    /// The version of the bootloader API.
    ///
    /// Automatically generated from the crate version. Checked on deserialization to
    /// ensure that the kernel and bootloader use the same API version, i.e. the same config
    /// and boot info format.
    pub(crate) version: ApiVersion,

    /// Configuration for (optional) page table mappings created by the bootloader.
    pub mappings: Mappings,

    /// The size of the stack that the bootloader should allocate for the kernel (in bytes).
    ///
    /// The bootloader starts the kernel with a valid stack pointer. This setting defines
    /// the stack size that the bootloader should allocate and map. The stack is created
    /// with a guard page, so a stack overflow will lead to a page fault.
    pub kernel_stack_size: u64,

    /// Configuration for the frame buffer that can be used by the kernel to display pixels
    /// on the screen.
    pub frame_buffer: FrameBuffer,
}

impl BootloaderConfig {
    pub(crate) const UUID: [u8; 16] = [
        0x74, 0x3C, 0xA9, 0x61, 0x09, 0x36, 0x46, 0xA0, 0xBB, 0x55, 0x5C, 0x15, 0x89, 0x15, 0x25,
        0x3D,
    ];
    #[doc(hidden)]
    pub const SERIALIZED_LEN: usize = 96;

    /// Creates a new default configuration with the following values:
    ///
    /// - `kernel_stack_size`: 80kiB
    /// - `mappings`: See [`Mappings::new_default()`]
    /// - `frame_buffer`: See [`FrameBuffer::new_default()`]
    pub const fn new_default() -> Self {
        Self {
            kernel_stack_size: 80 * 1024,
            version: ApiVersion::new_default(),
            mappings: Mappings::new_default(),
            frame_buffer: FrameBuffer::new_default(),
        }
    }

    /// Serializes the configuration to a byte array.
    ///
    /// This is used by the [`crate::entry_point`] macro to store the configuration in a
    /// dedicated section in the resulting ELF file.
    pub const fn serialize(&self) -> [u8; Self::SERIALIZED_LEN] {
        let Self {
            version,
            mappings,
            kernel_stack_size,
            frame_buffer,
        } = self;
        let ApiVersion {
            version_major,
            version_minor,
            version_patch,
            pre_release,
        } = version;
        let Mappings {
            kernel_stack,
            boot_info,
            framebuffer,
            physical_memory,
            page_table_recursive,
        } = mappings;
        let FrameBuffer {
            minimum_framebuffer_height,
            minimum_framebuffer_width,
        } = frame_buffer;

        let version = {
            let one = concat_2_2(version_major.to_le_bytes(), version_minor.to_le_bytes());
            let two = concat_2_1(version_patch.to_le_bytes(), [*pre_release as u8]);
            concat_4_3(one, two)
        };
        let buf = concat_16_7(Self::UUID, version);
        let buf = concat_23_8(buf, kernel_stack_size.to_le_bytes());

        let buf = concat_31_9(buf, kernel_stack.serialize());
        let buf = concat_40_9(buf, boot_info.serialize());
        let buf = concat_49_9(buf, framebuffer.serialize());

        let buf = concat_58_10(
            buf,
            match physical_memory {
                Option::None => [0; 10],
                Option::Some(m) => concat_1_9([1], m.serialize()),
            },
        );
        let buf = concat_68_10(
            buf,
            match page_table_recursive {
                Option::None => [0; 10],
                Option::Some(m) => concat_1_9([1], m.serialize()),
            },
        );
        let buf = concat_78_9(
            buf,
            match minimum_framebuffer_height {
                Option::None => [0; 9],
                Option::Some(addr) => concat_1_8([1], addr.to_le_bytes()),
            },
        );
        let buf = concat_87_9(
            buf,
            match minimum_framebuffer_width {
                Option::None => [0; 9],
                Option::Some(addr) => concat_1_8([1], addr.to_le_bytes()),
            },
        );

        buf
    }

    /// Tries to deserialize a config byte array that was created using [`Self::serialize`].
    ///
    /// This is used by the bootloader to deserialize the configuration given in the kernel's
    /// ELF file.
    ///
    /// TODO: return error enum
    pub fn deserialize(serialized: &[u8]) -> Result<Self, ()> {
        if serialized.len() != Self::SERIALIZED_LEN {
            return Err(());
        }

        let s = serialized;

        let (uuid, s) = split_array_ref(s);
        if uuid != &Self::UUID {
            return Err(());
        }

        let (version, s) = {
            let (&major, s) = split_array_ref(s);
            let (&minor, s) = split_array_ref(s);
            let (&patch, s) = split_array_ref(s);
            let (&pre, s) = split_array_ref(s);
            let pre = match pre {
                [0] => false,
                [1] => true,
                _ => return Err(()),
            };

            let version = ApiVersion {
                version_major: u16::from_le_bytes(major),
                version_minor: u16::from_le_bytes(minor),
                version_patch: u16::from_le_bytes(patch),
                pre_release: pre,
            };
            (version, s)
        };

        // TODO check version against this crate version -> error if they're different

        let (&kernel_stack_size, s) = split_array_ref(s);

        let (mappings, s) = {
            let (&kernel_stack, s) = split_array_ref(s);
            let (&boot_info, s) = split_array_ref(s);
            let (&framebuffer, s) = split_array_ref(s);
            let (&physical_memory_some, s) = split_array_ref(s);
            let (&physical_memory, s) = split_array_ref(s);
            let (&page_table_recursive_some, s) = split_array_ref(s);
            let (&page_table_recursive, s) = split_array_ref(s);

            let mappings = Mappings {
                kernel_stack: Mapping::deserialize(&kernel_stack)?,
                boot_info: Mapping::deserialize(&boot_info)?,
                framebuffer: Mapping::deserialize(&framebuffer)?,
                physical_memory: match physical_memory_some {
                    [0] if physical_memory == [0; 9] => Option::None,
                    [1] => Option::Some(Mapping::deserialize(&physical_memory)?),
                    _ => return Err(()),
                },
                page_table_recursive: match page_table_recursive_some {
                    [0] if page_table_recursive == [0; 9] => Option::None,
                    [1] => Option::Some(Mapping::deserialize(&page_table_recursive)?),
                    _ => return Err(()),
                },
            };
            (mappings, s)
        };

        let (frame_buffer, s) = {
            let (&min_framebuffer_height_some, s) = split_array_ref(s);
            let (&min_framebuffer_height, s) = split_array_ref(s);
            let (&min_framebuffer_width_some, s) = split_array_ref(s);
            let (&min_framebuffer_width, s) = split_array_ref(s);

            let frame_buffer = FrameBuffer {
                minimum_framebuffer_height: match min_framebuffer_height_some {
                    [0] if min_framebuffer_height == [0; 8] => Option::None,
                    [1] => Option::Some(u64::from_le_bytes(min_framebuffer_height)),
                    _ => return Err(()),
                },
                minimum_framebuffer_width: match min_framebuffer_width_some {
                    [0] if min_framebuffer_width == [0; 8] => Option::None,
                    [1] => Option::Some(u64::from_le_bytes(min_framebuffer_width)),
                    _ => return Err(()),
                },
            };
            (frame_buffer, s)
        };

        if !s.is_empty() {
            return Err(());
        }

        Ok(Self {
            version,
            kernel_stack_size: u64::from_le_bytes(kernel_stack_size),
            mappings,
            frame_buffer,
        })
    }

    #[cfg(test)]
    fn random() -> Self {
        Self {
            version: ApiVersion::random(),
            mappings: Mappings::random(),
            kernel_stack_size: rand::random(),
            frame_buffer: FrameBuffer::random(),
        }
    }
}

impl Default for BootloaderConfig {
    fn default() -> Self {
        Self::new_default()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ApiVersion {
    /// Bootloader version (major).
    version_major: u16,
    /// Bootloader version (minor).
    version_minor: u16,
    /// Bootloader version (patch).
    version_patch: u16,
    /// Whether the bootloader version is a pre-release.
    ///
    /// We can't store the full prerelease string of the version number since it could be
    /// arbitrarily long.
    pre_release: bool,
}

impl ApiVersion {
    const fn new_default() -> Self {
        Self {
            // todo: generate these from build script
            version_major: 0,
            version_minor: 0,
            version_patch: 0,
            pre_release: true,
        }
    }

    #[cfg(test)]
    fn random() -> ApiVersion {
        Self {
            version_major: rand::random(),
            version_minor: rand::random(),
            version_patch: rand::random(),
            pre_release: rand::random(),
        }
    }
}

impl Default for ApiVersion {
    fn default() -> Self {
        Self::new_default()
    }
}

/// Allows to configure the virtual memory mappings created by the bootloader.
#[derive(Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Mappings {
    /// Configures how the kernel stack should be mapped.
    pub kernel_stack: Mapping,
    /// Specifies where the [`crate::BootInfo`] struct should be placed in virtual memory.
    pub boot_info: Mapping,
    /// Specifies the mapping of the frame buffer memory region.
    pub framebuffer: Mapping,
    /// The bootloader supports to map the whole physical memory into the virtual address
    /// space at some offset. This is useful for accessing and modifying the page tables set
    /// up by the bootloader.
    ///
    /// Defaults to `None`, i.e. no mapping of the physical memory.
    pub physical_memory: Option<Mapping>,
    /// As an alternative to mapping the whole physical memory (see [`Self::physical_memory`]),
    /// the bootloader also has support for setting up a
    /// [recursive level 4 page table](https://os.phil-opp.com/paging-implementation/#recursive-page-tables).
    ///
    /// Defaults to `None`, i.e. no recursive mapping.
    pub page_table_recursive: Option<Mapping>,
}

impl Mappings {
    /// Creates a new mapping configuration with dynamic mapping for kernel, boot info and
    /// frame buffer. Neither physical memory mapping nor recursive page table creation are
    /// enabled.
    pub const fn new_default() -> Self {
        Self {
            kernel_stack: Mapping::new_default(),
            boot_info: Mapping::new_default(),
            framebuffer: Mapping::new_default(),
            physical_memory: Option::None,
            page_table_recursive: Option::None,
        }
    }

    #[cfg(test)]
    fn random() -> Mappings {
        let phys = rand::random();
        let recursive = rand::random();
        Self {
            kernel_stack: Mapping::random(),
            boot_info: Mapping::random(),
            framebuffer: Mapping::random(),
            physical_memory: if phys {
                Option::Some(Mapping::random())
            } else {
                Option::None
            },
            page_table_recursive: if recursive {
                Option::Some(Mapping::random())
            } else {
                Option::None
            },
        }
    }
}

/// Configuration for the frame buffer used for graphical output.
#[derive(Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct FrameBuffer {
    /// Instructs the bootloader to set up a framebuffer format that has at least the given height.
    ///
    /// If this is not possible, the bootloader will fall back to a smaller format.
    pub minimum_framebuffer_height: Option<u64>,
    /// Instructs the bootloader to set up a framebuffer format that has at least the given width.
    ///
    /// If this is not possible, the bootloader will fall back to a smaller format.
    pub minimum_framebuffer_width: Option<u64>,
}

impl FrameBuffer {
    /// Creates a default configuration without any requirements.
    pub const fn new_default() -> Self {
        Self {
            minimum_framebuffer_height: Option::None,
            minimum_framebuffer_width: Option::None,
        }
    }

    #[cfg(test)]
    fn random() -> FrameBuffer {
        Self {
            minimum_framebuffer_height: if rand::random() {
                Option::Some(rand::random())
            } else {
                Option::None
            },
            minimum_framebuffer_width: if rand::random() {
                Option::Some(rand::random())
            } else {
                Option::None
            },
        }
    }
}

/// Specifies how the bootloader should map a memory region into the virtual address space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mapping {
    /// Look for an unused virtual memory region at runtime.
    Dynamic,
    /// Try to map the region at the given virtual address.
    ///
    /// The given virtual address must be page-aligned.
    ///
    /// This setting can lead to runtime boot errors if the given address is not aligned,
    /// already in use, or invalid for other reasons.
    FixedAddress(u64),
}

impl Mapping {
    /// Creates a new [`Mapping::Dynamic`].
    ///
    /// This function has identical results as [`Default::default`], the only difference is
    /// that this is a `const` function.
    pub const fn new_default() -> Self {
        Self::Dynamic
    }

    #[cfg(test)]
    fn random() -> Mapping {
        let fixed = rand::random();
        if fixed {
            Self::Dynamic
        } else {
            Self::FixedAddress(rand::random())
        }
    }

    const fn serialize(&self) -> [u8; 9] {
        match self {
            Mapping::Dynamic => [0; 9],
            Mapping::FixedAddress(addr) => concat_1_8([1], addr.to_le_bytes()),
        }
    }

    fn deserialize(serialized: &[u8; 9]) -> Result<Self, ()> {
        let (&variant, s) = split_array_ref(serialized);
        let (&addr, s) = split_array_ref(s);
        if !s.is_empty() {
            return Err(());
        }

        match variant {
            [0] if addr == [0; 8] => Ok(Mapping::Dynamic),
            [1] => Ok(Mapping::FixedAddress(u64::from_le_bytes(addr))),
            _ => Err(()),
        }
    }
}

impl Default for Mapping {
    fn default() -> Self {
        Self::new_default()
    }
}

/// Taken from https://github.com/rust-lang/rust/blob/e100ec5bc7cd768ec17d75448b29c9ab4a39272b/library/core/src/slice/mod.rs#L1673-L1677
///
/// TODO replace with `split_array` feature in stdlib as soon as it's stabilized,
/// see https://github.com/rust-lang/rust/issues/90091
fn split_array_ref<const N: usize, T>(slice: &[T]) -> (&[T; N], &[T]) {
    let (a, b) = slice.split_at(N);
    // SAFETY: a points to [T; N]? Yes it's [T] of length N (checked by split_at)
    unsafe { (&*(a.as_ptr() as *const [T; N]), b) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapping_serde() {
        for _ in 0..1000 {
            let config = Mapping::random();
            assert_eq!(Mapping::deserialize(&config.serialize()), Ok(config));
        }
    }

    #[test]
    fn config_serde() {
        for _ in 0..1000 {
            let config = BootloaderConfig::random();
            assert_eq!(
                BootloaderConfig::deserialize(&config.serialize()),
                Ok(config)
            );
        }
    }
}
