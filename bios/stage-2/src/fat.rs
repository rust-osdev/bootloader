// based on https://crates.io/crates/mini_fat by https://github.com/gridbugs

use crate::disk::{AlignedBuffer, Read, Seek, SeekFrom};
use core::char::DecodeUtf16Error;

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
    _root_cluster: u32,
}

impl Bpb {
    fn parse<D: Read + Seek>(disk: &mut D) -> Self {
        disk.seek(SeekFrom::Start(0));
        let raw = unsafe { disk.read_exact(512) };

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
            _root_cluster: root_cluster,
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

    fn maximum_valid_cluster(&self) -> u32 {
        self.count_of_clusters() + 1
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

pub struct FileSystem<D> {
    disk: D,
    bpb: Bpb,
}

impl<D: Read + Seek> FileSystem<D> {
    pub fn parse(mut disk: D) -> Self {
        Self {
            bpb: Bpb::parse(&mut disk),
            disk,
        }
    }

    pub fn find_file_in_root_dir(
        &mut self,
        name: &str,
        buffer: &mut dyn AlignedBuffer,
    ) -> Option<File> {
        let mut root_entries = self.read_root_dir(buffer).filter_map(|e| e.ok());
        let raw_entry = root_entries.find(|e| e.eq_name(name))?;

        let entry = match raw_entry {
            RawDirectoryEntry::Normal(entry) => DirectoryEntry {
                short_name: entry.short_filename_main,
                short_name_extension: entry.short_filename_extension,
                long_name_1: &[],
                long_name_2: &[],
                long_name_3: &[],
                file_size: entry.file_size,
                first_cluster: entry.first_cluster,
                attributes: entry.attributes,
            },
            RawDirectoryEntry::LongName(long_name) => match root_entries.next() {
                Some(RawDirectoryEntry::LongName(_)) => unimplemented!(),
                Some(RawDirectoryEntry::Normal(entry)) => DirectoryEntry {
                    short_name: entry.short_filename_main,
                    short_name_extension: entry.short_filename_extension,
                    long_name_1: long_name.name_1,
                    long_name_2: long_name.name_2,
                    long_name_3: long_name.name_3,
                    file_size: entry.file_size,
                    first_cluster: entry.first_cluster,
                    attributes: entry.attributes,
                },
                None => {
                    panic!("next none");
                }
            },
        };

        if entry.is_directory() {
            None
        } else {
            Some(File {
                first_cluster: entry.first_cluster,
                file_size: entry.file_size,
            })
        }
    }

    fn read_root_dir<'a>(
        &'a mut self,
        buffer: &'a mut (dyn AlignedBuffer + 'a),
    ) -> impl Iterator<Item = Result<RawDirectoryEntry, ()>> + 'a {
        match self.bpb.fat_type() {
            FatType::Fat32 => {
                // self.bpb.root_cluster;
                unimplemented!();
            }
            FatType::Fat12 | FatType::Fat16 => {
                let root_directory_size = self.bpb.root_directory_size();

                self.disk
                    .seek(SeekFrom::Start(self.bpb.root_directory_offset()));
                self.disk.read_exact_into(root_directory_size, buffer);

                buffer
                    .slice()
                    .chunks(DIRECTORY_ENTRY_BYTES)
                    .take_while(|raw_entry| raw_entry[0] != END_OF_DIRECTORY_PREFIX)
                    .filter(|raw_entry| raw_entry[0] != UNUSED_ENTRY_PREFIX)
                    .map(RawDirectoryEntry::parse)
            }
        }
    }

    pub fn file_clusters<'a>(
        &'a mut self,
        file: &File,
    ) -> impl Iterator<Item = Result<Cluster, ()>> + 'a {
        Traverser {
            current_entry: file.first_cluster,
            bpb: &self.bpb,
            disk: &mut self.disk,
        }
    }
}

#[derive(Debug)]
pub struct Cluster {
    pub index: u32,
    pub start_offset: u64,
    pub len_bytes: u32,
}

struct Traverser<'a, D> {
    disk: &'a mut D,
    current_entry: u32,
    bpb: &'a Bpb,
}

impl<D> Traverser<'_, D>
where
    D: Read + Seek,
{
    fn next_cluster(&mut self) -> Result<Option<Cluster>, ()> {
        let entry = classify_fat_entry(
            self.bpb.fat_type(),
            self.current_entry,
            self.bpb.maximum_valid_cluster(),
        )
        .map_err(|_| ())?;
        let entry = match entry {
            FileFatEntry::AllocatedCluster(cluster) => cluster,
            FileFatEntry::EndOfFile => return Ok(None),
        };
        let cluster_start =
            self.bpb.data_offset() + (u64::from(entry) - 2) * self.bpb.bytes_per_cluster() as u64;
        let next_entry =
            fat_entry_of_nth_cluster(self.disk, self.bpb.fat_type(), self.bpb.fat_offset(), entry);
        let index = self.current_entry;
        self.current_entry = next_entry;

        Ok(Some(Cluster {
            index,
            start_offset: cluster_start,
            len_bytes: self.bpb.bytes_per_cluster(),
        }))
    }
}

impl<D> Iterator for Traverser<'_, D>
where
    D: Read + Seek,
{
    type Item = Result<Cluster, ()>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_cluster().transpose()
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

#[allow(dead_code)]
#[derive(Clone)]
pub struct DirectoryEntry<'a> {
    short_name: &'a str,
    short_name_extension: &'a str,
    long_name_1: &'a [u8],
    long_name_2: &'a [u8],
    long_name_3: &'a [u8],
    file_size: u32,
    first_cluster: u32,
    attributes: u8,
}

impl<'a> DirectoryEntry<'a> {
    pub fn is_directory(&self) -> bool {
        self.attributes & directory_attributes::DIRECTORY != 0
    }
}

#[derive(Debug)]
struct RawDirectoryEntryNormal<'a> {
    short_filename_main: &'a str,
    short_filename_extension: &'a str,
    attributes: u8,
    first_cluster: u32,
    file_size: u32,
}

#[allow(dead_code)]
#[derive(Debug)]
struct RawDirectoryEntryLongName<'a> {
    order: u8,
    name_1: &'a [u8],
    name_2: &'a [u8],
    name_3: &'a [u8],
    attributes: u8,
    checksum: u8,
}

impl<'a> RawDirectoryEntryLongName<'a> {
    pub fn name(&self) -> impl Iterator<Item = Result<char, DecodeUtf16Error>> + 'a {
        let iter = self
            .name_1
            .chunks(2)
            .chain(self.name_2.chunks(2))
            .chain(self.name_3.chunks(2))
            .map(|c| u16::from_le_bytes(c.try_into().unwrap()))
            .take_while(|&c| c != 0);
        char::decode_utf16(iter)
    }
}

#[derive(Debug)]
enum RawDirectoryEntry<'a> {
    Normal(RawDirectoryEntryNormal<'a>),
    LongName(RawDirectoryEntryLongName<'a>),
}

impl<'a> RawDirectoryEntry<'a> {
    fn parse(raw: &'a [u8]) -> Result<Self, ()> {
        let attributes = raw[11];
        if attributes == directory_attributes::LONG_NAME {
            let order = raw[0];
            let name_1 = &raw[1..11];
            let checksum = raw[13];
            let name_2 = &raw[14..26];
            let name_3 = &raw[28..32];

            Ok(Self::LongName(RawDirectoryEntryLongName {
                order,
                name_1,
                name_2,
                name_3,
                attributes,
                checksum,
            }))
        } else {
            fn slice_to_string(slice: &[u8]) -> Result<&str, ()> {
                const SKIP_SPACE: u8 = 0x20;
                let mut iter = slice.iter().copied();
                match iter.position(|c| c != SKIP_SPACE) {
                    Some(start_idx) => {
                        let end_idx =
                            start_idx + iter.position(|c| c == SKIP_SPACE).unwrap_or(slice.len());
                        core::str::from_utf8(&slice[start_idx..end_idx]).map_err(|_| ())
                    }
                    None => Ok(""),
                }
            }
            let short_filename_main = slice_to_string(&raw[0..8])?;
            let short_filename_extension = slice_to_string(&raw[8..11])?;
            let first_cluster_hi = u16::from_le_bytes(raw[20..22].try_into().unwrap());
            let first_cluster_lo = u16::from_le_bytes(raw[26..28].try_into().unwrap());
            let first_cluster = ((first_cluster_hi as u32) << 16) | (first_cluster_lo as u32);
            let file_size = u32::from_le_bytes(raw[28..32].try_into().unwrap());
            Ok(Self::Normal(RawDirectoryEntryNormal {
                short_filename_main,
                short_filename_extension,
                attributes,
                first_cluster,
                file_size,
            }))
        }
    }

    pub fn eq_name(&self, name: &str) -> bool {
        match self {
            RawDirectoryEntry::Normal(entry) => entry
                .short_filename_main
                .chars()
                .chain(entry.short_filename_extension.chars())
                .eq(name.chars()),
            RawDirectoryEntry::LongName(entry) => entry.name().eq(name.chars().map(Ok)),
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

fn fat_entry_of_nth_cluster<D>(disk: &mut D, fat_type: FatType, fat_start: u64, n: u32) -> u32
where
    D: Seek + Read,
{
    debug_assert!(n >= 2);
    match fat_type {
        FatType::Fat32 => {
            let base = n as u64 * 4;
            disk.seek(SeekFrom::Start(fat_start + base));
            let buf = unsafe { disk.read_exact(4) };
            let buf: [u8; 4] = buf.try_into().unwrap();
            u32::from_le_bytes(buf) & 0x0FFFFFFF
        }
        FatType::Fat16 => {
            let base = n as u64 * 2;
            disk.seek(SeekFrom::Start(fat_start + base));
            let buf = unsafe { disk.read_exact(2) };
            let buf: [u8; 2] = buf.try_into().unwrap();
            u16::from_le_bytes(buf) as u32
        }
        FatType::Fat12 => {
            let base = n as u64 + (n as u64 / 2);
            disk.seek(SeekFrom::Start(fat_start + base));
            let buf = unsafe { disk.read_exact(2) };
            let buf: [u8; 2] = buf.try_into().unwrap();
            let entry16 = u16::from_le_bytes(buf);
            if n & 1 == 0 {
                (entry16 & 0xFFF) as u32
            } else {
                (entry16 >> 4) as u32
            }
        }
    }
}
