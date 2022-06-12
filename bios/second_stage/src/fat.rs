// based on https://crates.io/crates/mini_fat

use crate::disk::Read;

const DIRECTORY_ENTRY_BYTES: usize = 32;
const UNUSED_ENTRY_PREFIX: u8 = 0xE5;
const END_OF_DIRECTORY_PREFIX: u8 = 0;

pub struct File {
    first_cluster: u32,
    file_size: u32,
}

impl File {
    pub fn file_size(&self) -> u32 {
        self.file_size
    }
}

struct Bpb {
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sector_count: u16,
    num_fats: u8,
    root_entry_count: u16,
    total_sectors_16: u16,
    fat_size_16: u16,
    total_sectors_32: u32,
    fat_size_32: u32,
    root_cluster: u32,
}

impl Bpb {
    fn parse<D: Read>(disk: &mut D) -> Self {
        let mut raw = [0u8; 512];
        disk.read_exact(&mut raw);

        let bytes_per_sector = u16::from_le_bytes(raw[11..13].try_into().unwrap());
        let sectors_per_cluster = raw[13];
        let reserved_sector_count = u16::from_le_bytes(raw[14..16].try_into().unwrap());
        let num_fats = raw[16];
        let root_entry_count = u16::from_le_bytes(raw[17..19].try_into().unwrap());
        let fat_size_16 = u16::from_le_bytes(raw[22..24].try_into().unwrap());

        let total_sectors_16 = u16::from_le_bytes(raw[19..21].try_into().unwrap());
        let total_sectors_32 = u32::from_le_bytes(raw[32..36].try_into().unwrap());

        let root_cluster;
        let fat_size_32;

        if (total_sectors_16 == 0) && (total_sectors_32 != 0) {
            // FAT32
            fat_size_32 = u32::from_le_bytes(raw[36..40].try_into().unwrap());
            root_cluster = u32::from_le_bytes(raw[44..48].try_into().unwrap());
        } else if (total_sectors_16 != 0) && (total_sectors_32 == 0) {
            // FAT12 or FAT16
            fat_size_32 = 0;
            root_cluster = 0;
        } else {
            panic!("ExactlyOneTotalSectorsFieldMustBeZero");
        }

        Self {
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sector_count,
            num_fats,
            root_entry_count,
            total_sectors_16,
            fat_size_16,
            total_sectors_32,
            fat_size_32,
            root_cluster,
        }
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

    fn root_directory_size(&self) -> usize {
        if self.fat_type() == FatType::Fat32 {
            debug_assert_eq!(self.root_entry_count, 0);
        }
        self.root_entry_count as usize * DIRECTORY_ENTRY_BYTES
    }

    fn root_directory_offset(&self) -> u64 {
        (self.reserved_sector_count as u64 + (self.num_fats as u64 * self.fat_size_16 as u64))
            * self.bytes_per_sector as u64
    }
}

pub struct FileSystem<D> {
    disk: D,
    bpb: Bpb,
}

impl<D: Read> FileSystem<D> {
    pub fn parse(mut disk: D) -> Self {
        Self {
            bpb: Bpb::parse(&mut disk),
            disk,
        }
    }

    pub fn lookup_file(&mut self, path: &str) -> Option<File> {
        todo!();
    }

    fn read_root_dir(&mut self) {
        match self.bpb.fat_type() {
            FatType::Fat32 => {
                self.bpb.root_cluster;
            }
            FatType::Fat12 | FatType::Fat16 => {
                self.bpb.root_directory_offset();
                self.bpb.root_directory_size();
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FatType {
    Fat12,
    Fat16,
    Fat32,
}
