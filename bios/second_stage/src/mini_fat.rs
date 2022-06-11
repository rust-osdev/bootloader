// based on https://crates.io/crates/mini_fat

use core::ops::Range;

#[derive(Debug)]
pub enum Error {
    UnexpectedNonZero {
        byte_index: usize,
    },
    ExactlyOneTotalSectorsFieldMustBeZero {
        total_sectors_16: u16,
        total_sectors_32: u32,
    },
    ExactlyOneFatSizeMustBeZero {
        fat_size_16: u16,
        fat_size_32: u32,
    },
    InvalidSignature(u16),
    InvalidFatEntry(u32),
    FatLookup(FatLookupError),
    NoSuchFile,
    InvalidPath,
    ExpectedFileFoundDirectory,
}

#[derive(Debug)]
pub struct Bpb<'a> {
    jmp_boot: [u8; 3],
    oem_name: &'a [u8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sector_count: u16,
    num_fats: u8,
    root_entry_count: u16,
    total_sectors_16: u16,
    media: u8,
    fat_size_16: u16,
    sectors_per_track: u16,
    num_heads: u16,
    hidden_sectors: u32,
    total_sectors_32: u32,
    fat_size_32: u32,
    ext_flags: u16,
    fs_version: u16,
    root_cluster: u32,
    fs_info: u16,
    bk_boot_sector: u16,
    drive_number: u8,
    boot_signature: u8,
    volume_id: u32,
    volume_label: &'a [u8],
    file_system_type: &'a [u8],
    signature: u16,
}

const BPB_SIZE: usize = 512;
const REQUIRED_SIGNATURE: u16 = 0xAA55;

impl<'a> Bpb<'a> {
    pub fn parse(raw: &'a [u8]) -> Result<Self, Error> {
        let jmp_boot = [raw[0], raw[1], raw[2]];
        let oem_name = &raw[3..11];
        let bytes_per_sector = u16::from_le_bytes(raw[11..13].try_into().unwrap());
        let sectors_per_cluster = raw[13];
        let reserved_sector_count = u16::from_le_bytes(raw[14..16].try_into().unwrap());
        let num_fats = raw[16];
        let root_entry_count = u16::from_le_bytes(raw[17..19].try_into().unwrap());
        let total_sectors_16 = u16::from_le_bytes(raw[19..21].try_into().unwrap());
        let media = raw[21];
        let fat_size_16 = u16::from_le_bytes(raw[22..24].try_into().unwrap());
        panic!("baz");
        let sectors_per_track = u16::from_le_bytes(raw[24..26].try_into().unwrap());
        let num_heads = u16::from_le_bytes(raw[26..28].try_into().unwrap());
        let hidden_sectors = u32::from_le_bytes(raw[28..32].try_into().unwrap());
        let total_sectors_32 = u32::from_le_bytes(raw[32..36].try_into().unwrap());

        let (
            fat_size_32,
            ext_flags,
            fs_version,
            root_cluster,
            fs_info,
            bk_boot_sector,
            drive_number,
            boot_signature,
            volume_id,
            volume_label,
            file_system_type,
        );
        if (total_sectors_16 == 0) && (total_sectors_32 != 0) {
            // FAT32
            fat_size_32 = u32::from_le_bytes(raw[36..40].try_into().unwrap());
            ext_flags = u16::from_le_bytes(raw[40..42].try_into().unwrap());
            fs_version = u16::from_le_bytes(raw[42..44].try_into().unwrap());
            root_cluster = u32::from_le_bytes(raw[44..48].try_into().unwrap());
            fs_info = u16::from_le_bytes(raw[48..50].try_into().unwrap());
            bk_boot_sector = u16::from_le_bytes(raw[50..52].try_into().unwrap());
            for i in 52..64 {
                if raw[i] != 0 {
                    return Err(Error::UnexpectedNonZero { byte_index: i });
                }
            }
            drive_number = raw[64];
            if raw[65] != 0 {
                return Err(Error::UnexpectedNonZero { byte_index: 65 });
            }
            boot_signature = raw[66];
            volume_id = u32::from_le_bytes(raw[67..71].try_into().unwrap());
            volume_label = &raw[71..82];
            file_system_type = &raw[82..90];
        } else if (total_sectors_16 != 0) && (total_sectors_32 == 0) {
            // FAT12 or FAT16
            fat_size_32 = 0;
            ext_flags = 0;
            fs_version = 0;
            root_cluster = 0;
            fs_info = 0;
            bk_boot_sector = 0;
            drive_number = raw[36];
            if raw[37] != 0 {
                return Err(Error::UnexpectedNonZero { byte_index: 37 });
            }
            boot_signature = raw[38];
            volume_id = u32::from_le_bytes(raw[39..43].try_into().unwrap());
            volume_label = &raw[43..54];
            file_system_type = &raw[54..62];
        } else {
            return Err(Error::ExactlyOneTotalSectorsFieldMustBeZero {
                total_sectors_16,
                total_sectors_32,
            });
        }
        if (fat_size_16 == 0) == (fat_size_32 == 0) {
            return Err(Error::ExactlyOneFatSizeMustBeZero {
                fat_size_16,
                fat_size_32,
            });
        }
        let signature = u16::from_le_bytes(raw[510..512].try_into().unwrap());
        if signature != REQUIRED_SIGNATURE {
            return Err(Error::InvalidSignature(signature));
        }
        Ok(Self {
            jmp_boot,
            oem_name,
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sector_count,
            num_fats,
            root_entry_count,
            total_sectors_16,
            media,
            fat_size_16,
            sectors_per_track,
            num_heads,
            hidden_sectors,
            total_sectors_32,
            fat_size_32,
            ext_flags,
            fs_version,
            root_cluster,
            fs_info,
            bk_boot_sector,
            drive_number,
            boot_signature,
            volume_id,
            volume_label,
            file_system_type,
            signature,
        })
    }

    fn fat_size_in_sectors(&self) -> u32 {
        if self.fat_size_16 != 0 && self.fat_size_32 == 0 {
            self.fat_size_16 as u32
        } else {
            debug_assert!(self.fat_size_16 == 0 && self.fat_size_32 != 0);
            self.fat_size_32
        }
    }

    fn count_of_clusters(&self) -> u32 {
        let root_dir_sectors = ((self.root_entry_count as u32 * 32)
            + (self.bytes_per_sector as u32 - 1))
            / self.bytes_per_sector as u32;
        let total_sectors = if self.total_sectors_16 != 0 {
            self.total_sectors_16 as u32
        } else {
            self.total_sectors_32
        };
        let data_sectors = total_sectors
            - (self.reserved_sector_count as u32
                + (self.num_fats as u32 * self.fat_size_in_sectors())
                + root_dir_sectors);
        data_sectors / self.sectors_per_cluster as u32
    }

    fn fat_type(&self) -> FatType {
        let count_of_clusters = self.count_of_clusters();
        if count_of_clusters < 4085 {
            FatType::Fat12
        } else if count_of_clusters < 65525 {
            FatType::Fat16
        } else {
            FatType::Fat32
        }
    }

    fn maximum_valid_cluster(&self) -> u32 {
        self.count_of_clusters() + 1
    }

    fn root_directory_size(&self) -> usize {
        debug_assert!((self.fat_type() == FatType::Fat32) == (self.root_entry_count == 0));
        self.root_entry_count as usize * DIRECTORY_ENTRY_BYTES
    }

    fn root_directory_offset(&self) -> u64 {
        (self.reserved_sector_count as u64 + (self.num_fats as u64 * self.fat_size_16 as u64))
            * self.bytes_per_sector as u64
    }

    fn fat_offset(&self) -> u64 {
        self.reserved_sector_count as u64 * self.bytes_per_sector as u64
    }

    fn data_offset(&self) -> u64 {
        self.root_directory_size() as u64
            + ((self.reserved_sector_count as u64
                + self.fat_size_in_sectors() as u64 * self.num_fats as u64)
                * self.bytes_per_sector as u64)
    }

    pub fn bytes_per_cluster(&self) -> u32 {
        self.bytes_per_sector as u32 * self.sectors_per_cluster as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FatType {
    Fat12,
    Fat16,
    Fat32,
}

impl FatType {
    fn fat_entry_defective(self) -> u32 {
        match self {
            Self::Fat12 => 0xFF7,
            Self::Fat16 => 0xFFF7,
            Self::Fat32 => 0x0FFFFFF7,
        }
    }
}

mod directory_attributes {
    pub const READ_ONLY: u8 = 0x01;
    pub const HIDDEN: u8 = 0x02;
    pub const SYSTEM: u8 = 0x04;
    pub const VOLUME_ID: u8 = 0x08;
    pub const DIRECTORY: u8 = 0x10;

    pub const LONG_NAME: u8 = READ_ONLY | HIDDEN | SYSTEM | VOLUME_ID;
}

#[derive(Debug)]
pub enum FatLookupError {
    FreeCluster,
    DefectiveCluster,
    UnspecifiedEntryOne,
    ReservedEntry,
}

enum FileFatEntry {
    AllocatedCluster(u32),
    EndOfFile,
}

fn classify_fat_entry(
    fat_type: FatType,
    entry: u32,
    maximum_valid_cluster: u32,
) -> Result<FileFatEntry, FatLookupError> {
    match entry {
        0 => Err(FatLookupError::FreeCluster),
        1 => Err(FatLookupError::UnspecifiedEntryOne),
        entry => {
            if entry <= maximum_valid_cluster {
                Ok(FileFatEntry::AllocatedCluster(entry))
            } else if entry < fat_type.fat_entry_defective() {
                Err(FatLookupError::ReservedEntry)
            } else if entry == fat_type.fat_entry_defective() {
                Err(FatLookupError::DefectiveCluster)
            } else {
                Ok(FileFatEntry::EndOfFile)
            }
        }
    }
}

fn handle_read<H>(handle: &mut H, offset: u64, size: usize, buf: &mut [u8]) -> Result<(), Error>
where
    H: fatfs::Seek + fatfs::Read,
{
    handle.seek(fatfs::SeekFrom::Start(offset)).unwrap();
    handle.read_exact(buf).unwrap();
    Ok(())
}

fn fat_entry_of_nth_cluster<H>(
    handle: &mut H,
    fat_type: FatType,
    fat_start: u64,
    n: u32,
) -> Result<u32, Error>
where
    H: fatfs::Seek + fatfs::Read,
{
    debug_assert!(n >= 2);
    match fat_type {
        FatType::Fat32 => {
            let base = n as u64 * 4;
            handle
                .seek(fatfs::SeekFrom::Start(fat_start + base))
                .unwrap();
            let mut buf = [0; 4];
            handle.read_exact(&mut buf).unwrap();
            Ok(u32::from_le_bytes(buf) & 0x0FFFFFFF)
        }
        FatType::Fat16 => {
            let base = n as u64 * 2;
            handle
                .seek(fatfs::SeekFrom::Start(fat_start + base))
                .unwrap();
            let mut buf = [0; 2];
            handle.read_exact(&mut buf).unwrap();
            Ok(u16::from_le_bytes(buf) as u32)
        }
        FatType::Fat12 => {
            let base = n as u64 + (n as u64 / 2);
            handle
                .seek(fatfs::SeekFrom::Start(fat_start + base))
                .unwrap();
            let mut buf = [0; 2];
            handle.read_exact(&mut buf).unwrap();
            let entry16 = u16::from_le_bytes(buf);
            if n & 1 == 0 {
                Ok((entry16 & 0xFFF) as u32)
            } else {
                Ok((entry16 >> 4) as u32)
            }
        }
    }
}

fn read_bpb<'a, H>(
    handle: &mut H,
    partition_byte_start: u64,
    buf: &'a mut [u8],
) -> Result<Bpb<'a>, Error>
where
    H: fatfs::Seek + fatfs::Read,
{
    handle_read(handle, partition_byte_start, BPB_SIZE, buf)?;
    Bpb::parse(buf)
}

const DIRECTORY_ENTRY_BYTES: usize = 32;
const UNUSED_ENTRY_PREFIX: u8 = 0xE5;
const END_OF_DIRECTORY_PREFIX: u8 = 0;
