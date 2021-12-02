use crate::concat::*;

#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct BootloaderConfig {
    pub(crate) version: Version,

    pub mappings: Mappings,

    pub kernel_stack_size: u64,

    pub frame_buffer: FrameBuffer,
}

impl BootloaderConfig {
    pub(crate) const UUID: [u8; 16] = [
        0x74, 0x3C, 0xA9, 0x61, 0x09, 0x36, 0x46, 0xA0, 0xBB, 0x55, 0x5C, 0x15, 0x89, 0x15, 0x25,
        0x3D,
    ];
    pub const SERIALIZED_LEN: usize = 96;

    pub const fn new_default() -> Self {
        Self {
            kernel_stack_size: 80 * 1024,
            version: Version::new_default(),
            mappings: Mappings::new_default(),
            frame_buffer: FrameBuffer::new_default(),
        }
    }

    pub const fn serialize(&self) -> [u8; Self::SERIALIZED_LEN] {
        let Self {
            version,
            mappings,
            kernel_stack_size,
            frame_buffer,
        } = self;
        let Version {
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

    pub fn deserialize(serialized: &[u8]) -> Result<Self, ()> {
        if serialized.len() != Self::SERIALIZED_LEN {
            return Err(());
        }

        let s = serialized;

        let (uuid, s) = s.split_array_ref();
        if uuid != &Self::UUID {
            return Err(());
        }

        let (version, s) = {
            let (&major, s) = s.split_array_ref();
            let (&minor, s) = s.split_array_ref();
            let (&patch, s) = s.split_array_ref();
            let (&pre, s) = s.split_array_ref();
            let pre = match pre {
                [0] => false,
                [1] => true,
                _ => return Err(()),
            };

            let version = Version {
                version_major: u16::from_le_bytes(major),
                version_minor: u16::from_le_bytes(minor),
                version_patch: u16::from_le_bytes(patch),
                pre_release: pre,
            };
            (version, s)
        };

        let (&kernel_stack_size, s) = s.split_array_ref();

        let (mappings, s) = {
            let (&kernel_stack, s) = s.split_array_ref();
            let (&boot_info, s) = s.split_array_ref();
            let (&framebuffer, s) = s.split_array_ref();
            let (&physical_memory_some, s) = s.split_array_ref();
            let (&physical_memory, s) = s.split_array_ref();
            let (&page_table_recursive_some, s) = s.split_array_ref();
            let (&page_table_recursive, s) = s.split_array_ref();

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
            let (&min_framebuffer_height_some, s) = s.split_array_ref();
            let (&min_framebuffer_height, s) = s.split_array_ref();
            let (&min_framebuffer_width_some, s) = s.split_array_ref();
            let (&min_framebuffer_width, s) = s.split_array_ref();

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
            version: Version::random(),
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
pub struct Version {
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

impl Version {
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
    fn random() -> Version {
        Self {
            version_major: rand::random(),
            version_minor: rand::random(),
            version_patch: rand::random(),
            pre_release: rand::random(),
        }
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::new_default()
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Mappings {
    pub kernel_stack: Mapping,
    pub boot_info: Mapping,
    pub framebuffer: Mapping,
    pub physical_memory: Option<Mapping>,
    pub page_table_recursive: Option<Mapping>,
}

impl Mappings {
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

#[derive(Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct FrameBuffer {
    pub minimum_framebuffer_height: Option<u64>,
    pub minimum_framebuffer_width: Option<u64>,
}

impl FrameBuffer {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mapping {
    Dynamic,
    FixedAddress(u64),
}

impl Mapping {
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

    pub fn deserialize(serialized: &[u8; 9]) -> Result<Self, ()> {
        let (&variant, s) = serialized.split_array_ref();
        let (&addr, s) = s.split_array_ref();
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
