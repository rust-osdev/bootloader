// based on https://github.com/rafalh/rust-fatfs/

use core::cmp;
use core::u16;
use core::u8;

use fatfs::{FatType, FormatVolumeOptions, FsStatusFlags};
use fatfs::{Read, Write};

const RESERVED_FAT_ENTRIES: u32 = 2;
// Size of single directory entry in bytes
pub(crate) const DIR_ENTRY_SIZE: u32 = 32;

const BITS_PER_BYTE: u32 = 8;
const KB_32: u32 = 1024;
const KB_64: u64 = 1024;
const MB_64: u64 = KB_64 * 1024;
const GB_64: u64 = MB_64 * 1024;

#[derive(Default, Debug, Clone)]
pub(crate) struct BiosParameterBlock {
    pub(crate) bytes_per_sector: u16,
    pub(crate) sectors_per_cluster: u8,
    pub(crate) reserved_sectors: u16,
    pub(crate) fats: u8,
    pub(crate) root_entries: u16,
    pub(crate) total_sectors_16: u16,
    pub(crate) media: u8,
    pub(crate) sectors_per_fat_16: u16,
    pub(crate) sectors_per_track: u16,
    pub(crate) heads: u16,
    pub(crate) hidden_sectors: u32,
    pub(crate) total_sectors_32: u32,

    // Extended BIOS Parameter Block
    pub(crate) sectors_per_fat_32: u32,
    pub(crate) extended_flags: u16,
    pub(crate) fs_version: u16,
    pub(crate) root_dir_first_cluster: u32,
    pub(crate) fs_info_sector: u16,
    pub(crate) backup_boot_sector: u16,
    pub(crate) reserved_0: [u8; 12],
    pub(crate) drive_num: u8,
    pub(crate) reserved_1: u8,
    pub(crate) ext_sig: u8,
    pub(crate) volume_id: u32,
    pub(crate) volume_label: [u8; 11],
    pub(crate) fs_type_label: [u8; 8],
}

impl BiosParameterBlock {
    fn deserialize<R: Read>(rdr: &mut R) -> Result<Self, R::Error> {
        let mut bpb = Self {
            bytes_per_sector: rdr.read_u16_le()?,
            sectors_per_cluster: rdr.read_u8()?,
            reserved_sectors: rdr.read_u16_le()?,
            fats: rdr.read_u8()?,
            root_entries: rdr.read_u16_le()?,
            total_sectors_16: rdr.read_u16_le()?,
            media: rdr.read_u8()?,
            sectors_per_fat_16: rdr.read_u16_le()?,
            sectors_per_track: rdr.read_u16_le()?,
            heads: rdr.read_u16_le()?,
            hidden_sectors: rdr.read_u32_le()?,
            total_sectors_32: rdr.read_u32_le()?,
            ..Self::default()
        };

        if bpb.is_fat32() {
            bpb.sectors_per_fat_32 = rdr.read_u32_le()?;
            bpb.extended_flags = rdr.read_u16_le()?;
            bpb.fs_version = rdr.read_u16_le()?;
            bpb.root_dir_first_cluster = rdr.read_u32_le()?;
            bpb.fs_info_sector = rdr.read_u16_le()?;
            bpb.backup_boot_sector = rdr.read_u16_le()?;
            rdr.read_exact(&mut bpb.reserved_0)?;
        }

        bpb.drive_num = rdr.read_u8()?;
        bpb.reserved_1 = rdr.read_u8()?;
        bpb.ext_sig = rdr.read_u8()?; // 0x29
        bpb.volume_id = rdr.read_u32_le()?;
        rdr.read_exact(&mut bpb.volume_label)?;
        rdr.read_exact(&mut bpb.fs_type_label)?;

        // when the extended boot signature is anything other than 0x29, the fields are invalid
        if bpb.ext_sig != 0x29 {
            // fields after ext_sig are not used - clean them
            bpb.volume_id = 0;
            bpb.volume_label = [0; 11];
            bpb.fs_type_label = [0; 8];
        }

        Ok(bpb)
    }

    fn serialize<W: Write>(&self, wrt: &mut W) -> Result<(), W::Error> {
        wrt.write_u16_le(self.bytes_per_sector)?;
        wrt.write_u8(self.sectors_per_cluster)?;
        wrt.write_u16_le(self.reserved_sectors)?;
        wrt.write_u8(self.fats)?;
        wrt.write_u16_le(self.root_entries)?;
        wrt.write_u16_le(self.total_sectors_16)?;
        wrt.write_u8(self.media)?;
        wrt.write_u16_le(self.sectors_per_fat_16)?;
        wrt.write_u16_le(self.sectors_per_track)?;
        wrt.write_u16_le(self.heads)?;
        wrt.write_u32_le(self.hidden_sectors)?;
        wrt.write_u32_le(self.total_sectors_32)?;

        if self.is_fat32() {
            wrt.write_u32_le(self.sectors_per_fat_32)?;
            wrt.write_u16_le(self.extended_flags)?;
            wrt.write_u16_le(self.fs_version)?;
            wrt.write_u32_le(self.root_dir_first_cluster)?;
            wrt.write_u16_le(self.fs_info_sector)?;
            wrt.write_u16_le(self.backup_boot_sector)?;
            wrt.write_all(&self.reserved_0)?;
        }

        wrt.write_u8(self.drive_num)?;
        wrt.write_u8(self.reserved_1)?;
        wrt.write_u8(self.ext_sig)?; // 0x29
        wrt.write_u32_le(self.volume_id)?;
        wrt.write_all(&self.volume_label)?;
        wrt.write_all(&self.fs_type_label)?;
        Ok(())
    }

    fn validate_bytes_per_sector(&self) -> Result<(), ()> {
        if self.bytes_per_sector.count_ones() != 1 {
            panic!(
                "invalid bytes_per_sector value in BPB: expected a power of two but got {}",
                self.bytes_per_sector
            );
            return Err(Error::CorruptedFileSystem);
        }
        if self.bytes_per_sector < 512 || self.bytes_per_sector > 4096 {
            panic!(
                "invalid bytes_per_sector value in BPB: expected value in range [512, 4096] but got {}",
                self.bytes_per_sector
            );
            return Err(Error::CorruptedFileSystem);
        }
        Ok(())
    }

    fn validate_sectors_per_cluster(&self) -> Result<(), ()> {
        if self.sectors_per_cluster.count_ones() != 1 {
            panic!(
                "invalid sectors_per_cluster value in BPB: expected a power of two but got {}",
                self.sectors_per_cluster
            );
            return Err(Error::CorruptedFileSystem);
        }
        if self.sectors_per_cluster < 1 || self.sectors_per_cluster > 128 {
            panic!(
                "invalid sectors_per_cluster value in BPB: expected value in range [1, 128] but got {}",
                self.sectors_per_cluster
            );
            return Err(Error::CorruptedFileSystem);
        }

        // bytes per sector is u16, sectors per cluster is u8, so guaranteed no overflow in multiplication
        let bytes_per_cluster =
            u32::from(self.bytes_per_sector) * u32::from(self.sectors_per_cluster);
        let maximum_compatibility_bytes_per_cluster: u32 = 32 * 1024;

        if bytes_per_cluster > maximum_compatibility_bytes_per_cluster {
            // 32k is the largest value to maintain greatest compatibility
            // Many implementations appear to support 64k per cluster, and some may support 128k or larger
            // However, >32k is not as thoroughly tested...
            // warn!("fs compatibility: bytes_per_cluster value '{}' in BPB exceeds '{}', and thus may be incompatible with some implementations",
            //     bytes_per_cluster, maximum_compatibility_bytes_per_cluster);
        }
        Ok(())
    }

    fn validate_reserved_sectors(&self) -> Result<(), ()> {
        let is_fat32 = self.is_fat32();
        if self.reserved_sectors < 1 {
            panic!(
                "invalid reserved_sectors value in BPB: {}",
                self.reserved_sectors
            );
            return Err(Error::CorruptedFileSystem);
        }
        if !is_fat32 && self.reserved_sectors != 1 {
            // Microsoft document indicates fat12 and fat16 code exists that presume this value is 1
            // warn!(
            //     "fs compatibility: reserved_sectors value '{}' in BPB is not '1', and thus is incompatible with some implementations",
            //     self.reserved_sectors
            // );
        }
        if is_fat32 && self.backup_boot_sector >= self.reserved_sectors {
            panic!(
                "Invalid BPB: expected backup boot-sector to be in the reserved region (sector < {}) but got sector {}",
                self.reserved_sectors, self.backup_boot_sector
            );
            return Err(Error::CorruptedFileSystem);
        }
        if is_fat32 && self.fs_info_sector >= self.reserved_sectors {
            panic!(
                "Invalid BPB: expected FSInfo sector to be in the reserved region (sector < {}) but got sector {}",
                self.reserved_sectors, self.fs_info_sector
            );
            return Err(Error::CorruptedFileSystem);
        }
        Ok(())
    }

    fn validate_fats(&self) -> Result<(), ()> {
        if self.fats == 0 {
            panic!("invalid fats value in BPB: {}", self.fats);
            return Err(Error::CorruptedFileSystem);
        }
        if self.fats > 2 {
            // Microsoft document indicates that few implementations support any values other than 1 or 2
            // warn!(
            //     "fs compatibility: numbers of FATs '{}' in BPB is greater than '2', and thus is incompatible with some implementations",
            //     self.fats
            // );
        }
        Ok(())
    }

    fn validate_root_entries(&self) -> Result<(), ()> {
        let is_fat32 = self.is_fat32();
        if is_fat32 && self.root_entries != 0 {
            panic!(
                "Invalid root_entries value in FAT32 BPB: expected 0 but got {}",
                self.root_entries
            );
            return Err(Error::CorruptedFileSystem);
        }
        if !is_fat32 && self.root_entries == 0 {
            panic!(
                "Invalid root_entries value in FAT12/FAT16 BPB: expected non-zero value but got {}",
                self.root_entries
            );
            return Err(Error::CorruptedFileSystem);
        }
        if (u32::from(self.root_entries) * DIR_ENTRY_SIZE) % u32::from(self.bytes_per_sector) != 0 {
            // warn!("Root entries should fill sectors fully");
        }
        Ok(())
    }

    fn validate_total_sectors(&self) -> Result<(), ()> {
        let is_fat32 = self.is_fat32();
        if is_fat32 && self.total_sectors_16 != 0 {
            panic!(
                "Invalid total_sectors_16 value in FAT32 BPB: expected 0 but got {}",
                self.total_sectors_16
            );
            return Err(Error::CorruptedFileSystem);
        }
        if (self.total_sectors_16 == 0) == (self.total_sectors_32 == 0) {
            panic!("Invalid BPB (total_sectors_16 or total_sectors_32 should be non-zero)");
            return Err(Error::CorruptedFileSystem);
        }
        let total_sectors = self.total_sectors();
        let first_data_sector = self.first_data_sector();
        if total_sectors <= first_data_sector {
            panic!(
                "Invalid total_sectors value in BPB: expected value > {} but got {}",
                first_data_sector, total_sectors
            );
            return Err(Error::CorruptedFileSystem);
        }
        Ok(())
    }

    fn validate_sectors_per_fat(&self) -> Result<(), ()> {
        let is_fat32 = self.is_fat32();
        if is_fat32 && self.sectors_per_fat_32 == 0 {
            panic!(
                "Invalid sectors_per_fat_32 value in FAT32 BPB: expected non-zero value but got {}",
                self.sectors_per_fat_32
            );
            return Err(Error::CorruptedFileSystem);
        }
        Ok(())
    }

    fn validate_total_clusters(&self) -> Result<(), ()> {
        let is_fat32 = self.is_fat32();
        let total_clusters = self.total_clusters();
        let fat_type = FatType::from_clusters(total_clusters);
        if is_fat32 != (fat_type == FatType::Fat32) {
            panic!("Invalid BPB: result of FAT32 determination from total number of clusters and sectors_per_fat_16 field differs");
            return Err(Error::CorruptedFileSystem);
        }
        if fat_type == FatType::Fat32 && total_clusters > 0x0FFF_FFFF {
            panic!("Invalid BPB: too many clusters {}", total_clusters);
            return Err(Error::CorruptedFileSystem);
        }

        let bits_per_fat_entry = fat_type.bits_per_fat_entry();
        let total_fat_entries =
            self.sectors_per_fat() * u32::from(self.bytes_per_sector) * 8 / bits_per_fat_entry;
        let usable_fat_entries = total_fat_entries - RESERVED_FAT_ENTRIES;
        if usable_fat_entries < total_clusters {
            // warn!(
            //     "FAT is too small (allows allocation of {} clusters) compared to the total number of clusters ({})",
            //     usable_fat_entries, total_clusters
            // );
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), ()> {
        if self.fs_version != 0 {
            panic!(
                "Unsupported filesystem version: expected 0 but got {}",
                self.fs_version
            );
            return Err(Error::CorruptedFileSystem);
        }
        self.validate_bytes_per_sector()?;
        self.validate_sectors_per_cluster()?;
        self.validate_reserved_sectors()?;
        self.validate_fats()?;
        self.validate_root_entries()?;
        self.validate_total_sectors()?;
        self.validate_sectors_per_fat()?;
        self.validate_total_clusters()?;
        Ok(())
    }

    pub(crate) fn mirroring_enabled(&self) -> bool {
        self.extended_flags & 0x80 == 0
    }

    pub(crate) fn active_fat(&self) -> u16 {
        // The zero-based number of the active FAT is only valid if mirroring is disabled.
        if self.mirroring_enabled() {
            0
        } else {
            self.extended_flags & 0x0F
        }
    }

    pub(crate) fn status_flags(&self) -> FsStatusFlags {
        FsStatusFlags::decode(self.reserved_1)
    }

    pub(crate) fn is_fat32(&self) -> bool {
        // because this field must be zero on FAT32, and
        // because it must be non-zero on FAT12/FAT16,
        // this provides a simple way to detect FAT32
        self.sectors_per_fat_16 == 0
    }

    pub(crate) fn sectors_per_fat(&self) -> u32 {
        if self.is_fat32() {
            self.sectors_per_fat_32
        } else {
            u32::from(self.sectors_per_fat_16)
        }
    }

    pub(crate) fn total_sectors(&self) -> u32 {
        if self.total_sectors_16 == 0 {
            self.total_sectors_32
        } else {
            u32::from(self.total_sectors_16)
        }
    }

    pub(crate) fn reserved_sectors(&self) -> u32 {
        u32::from(self.reserved_sectors)
    }

    pub(crate) fn root_dir_sectors(&self) -> u32 {
        let root_dir_bytes = u32::from(self.root_entries) * DIR_ENTRY_SIZE;
        (root_dir_bytes + u32::from(self.bytes_per_sector) - 1) / u32::from(self.bytes_per_sector)
    }

    pub(crate) fn sectors_per_all_fats(&self) -> u32 {
        u32::from(self.fats) * self.sectors_per_fat()
    }

    pub(crate) fn first_data_sector(&self) -> u32 {
        let root_dir_sectors = self.root_dir_sectors();
        let fat_sectors = self.sectors_per_all_fats();
        self.reserved_sectors() + fat_sectors + root_dir_sectors
    }

    pub(crate) fn total_clusters(&self) -> u32 {
        let total_sectors = self.total_sectors();
        let first_data_sector = self.first_data_sector();
        let data_sectors = total_sectors - first_data_sector;
        data_sectors / u32::from(self.sectors_per_cluster)
    }

    pub(crate) fn bytes_from_sectors(&self, sectors: u32) -> u64 {
        // Note: total number of sectors is a 32 bit number so offsets have to be 64 bit
        u64::from(sectors) * u64::from(self.bytes_per_sector)
    }

    pub(crate) fn sectors_from_clusters(&self, clusters: u32) -> u32 {
        // Note: total number of sectors is a 32 bit number so it should not overflow
        clusters * u32::from(self.sectors_per_cluster)
    }

    pub(crate) fn cluster_size(&self) -> u32 {
        u32::from(self.sectors_per_cluster) * u32::from(self.bytes_per_sector)
    }

    pub(crate) fn clusters_from_bytes(&self, bytes: u64) -> u32 {
        let cluster_size = u64::from(self.cluster_size());
        ((bytes + cluster_size - 1) / cluster_size) as u32
    }

    pub(crate) fn fs_info_sector(&self) -> u32 {
        u32::from(self.fs_info_sector)
    }

    pub(crate) fn backup_boot_sector(&self) -> u32 {
        u32::from(self.backup_boot_sector)
    }
}

pub(crate) struct BootSector {
    bootjmp: [u8; 3],
    oem_name: [u8; 8],
    pub(crate) bpb: BiosParameterBlock,
    boot_code: [u8; 448],
    boot_sig: [u8; 2],
}

impl BootSector {
    pub(crate) fn deserialize<R: Read>(rdr: &mut R) -> Result<Self, R::Error> {
        let mut boot = Self::default();
        rdr.read_exact(&mut boot.bootjmp)?;
        rdr.read_exact(&mut boot.oem_name)?;
        boot.bpb = BiosParameterBlock::deserialize(rdr)?;

        if boot.bpb.is_fat32() {
            rdr.read_exact(&mut boot.boot_code[0..420])?;
        } else {
            rdr.read_exact(&mut boot.boot_code[0..448])?;
        }
        rdr.read_exact(&mut boot.boot_sig)?;
        Ok(boot)
    }

    pub(crate) fn serialize<W: Write>(&self, wrt: &mut W) -> Result<(), W::Error> {
        wrt.write_all(&self.bootjmp)?;
        wrt.write_all(&self.oem_name)?;
        self.bpb.serialize(&mut *wrt)?;

        if self.bpb.is_fat32() {
            wrt.write_all(&self.boot_code[0..420])?;
        } else {
            wrt.write_all(&self.boot_code[0..448])?;
        }
        wrt.write_all(&self.boot_sig)?;
        Ok(())
    }

    pub(crate) fn validate(&self) -> Result<(), ()> {
        if self.boot_sig != [0x55, 0xAA] {
            panic!(
                "Invalid boot sector signature: expected [0x55, 0xAA] but got {:?}",
                self.boot_sig
            );
            return Err(Error::CorruptedFileSystem);
        }
        if self.bootjmp[0] != 0xEB && self.bootjmp[0] != 0xE9 {
            // warn!(
            //     "Unknown opcode {:x} in bootjmp boot sector field",
            //     self.bootjmp[0]
            // );
        }
        self.bpb.validate()?;
        Ok(())
    }
}

impl Default for BootSector {
    fn default() -> Self {
        Self {
            bootjmp: Default::default(),
            oem_name: Default::default(),
            bpb: BiosParameterBlock::default(),
            boot_code: [0; 448],
            boot_sig: Default::default(),
        }
    }
}

pub(crate) fn estimate_fat_type(total_bytes: u64) -> FatType {
    // Used only to select cluster size if FAT type has not been overriden in options
    if total_bytes < 4 * MB_64 {
        FatType::Fat12
    } else if total_bytes < 512 * MB_64 {
        FatType::Fat16
    } else {
        FatType::Fat32
    }
}

fn determine_bytes_per_cluster(
    total_bytes: u64,
    bytes_per_sector: u16,
    fat_type: Option<FatType>,
) -> u32 {
    const MAX_CLUSTER_SIZE: u32 = 32 * KB_32;

    let fat_type = fat_type.unwrap_or_else(|| estimate_fat_type(total_bytes));
    let bytes_per_cluster = match fat_type {
        FatType::Fat12 => (total_bytes.next_power_of_two() / MB_64 * 512) as u32,
        FatType::Fat16 => {
            if total_bytes <= 16 * MB_64 {
                KB_32
            } else if total_bytes <= 128 * MB_64 {
                2 * KB_32
            } else {
                ((total_bytes.next_power_of_two() / (64 * MB_64)) as u32) * KB_32
            }
        }
        FatType::Fat32 => {
            if total_bytes <= 260 * MB_64 {
                512
            } else if total_bytes <= 8 * GB_64 {
                4 * KB_32
            } else {
                ((total_bytes.next_power_of_two() / (2 * GB_64)) as u32) * KB_32
            }
        }
    };
    let bytes_per_cluster_clamped = cmp::min(
        cmp::max(bytes_per_cluster, u32::from(bytes_per_sector)),
        MAX_CLUSTER_SIZE,
    );
    debug_assert!(bytes_per_cluster_clamped.is_power_of_two());
    bytes_per_cluster_clamped
}

fn determine_sectors_per_fat(
    total_sectors: u32,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    fat_type: FatType,
    reserved_sectors: u16,
    root_dir_sectors: u32,
    fats: u8,
) -> u32 {
    //
    // FAT size formula transformations:
    //
    // Initial basic formula:
    // size of FAT in bits >= (total number of clusters + 2) * bits per FAT entry
    //
    // Note: when computing number of clusters from number of sectors rounding down is used because partial clusters
    // are not allowed
    // Note: in those transformations '/' is a floating-point division (not a rounding towards zero division)
    //
    // data_sectors = total_sectors - reserved_sectors - fats * sectors_per_fat - root_dir_sectors
    // total_clusters = floor(data_sectors / sectors_per_cluster)
    // bits_per_sector = bytes_per_sector * 8
    // sectors_per_fat * bits_per_sector >= (total_clusters + 2) * bits_per_fat_entry
    // sectors_per_fat * bits_per_sector >= (floor(data_sectors / sectors_per_cluster) + 2) * bits_per_fat_entry
    //
    // Note: omitting the floor function can cause the FAT to be bigger by 1 entry - negligible
    //
    // sectors_per_fat * bits_per_sector >= (data_sectors / sectors_per_cluster + 2) * bits_per_fat_entry
    // t0 = total_sectors - reserved_sectors - root_dir_sectors
    // sectors_per_fat * bits_per_sector >= ((t0 - fats * sectors_per_fat) / sectors_per_cluster + 2) * bits_per_fat_entry
    // sectors_per_fat * bits_per_sector / bits_per_fat_entry >= (t0 - fats * sectors_per_fat) / sectors_per_cluster + 2
    // sectors_per_fat * bits_per_sector / bits_per_fat_entry >= t0 / sectors_per_cluster + 2 - fats * sectors_per_fat / sectors_per_cluster
    // sectors_per_fat * bits_per_sector / bits_per_fat_entry + fats * sectors_per_fat / sectors_per_cluster >= t0 / sectors_per_cluster + 2
    // sectors_per_fat * (bits_per_sector / bits_per_fat_entry + fats / sectors_per_cluster) >= t0 / sectors_per_cluster + 2
    // sectors_per_fat >= (t0 / sectors_per_cluster + 2) / (bits_per_sector / bits_per_fat_entry + fats / sectors_per_cluster)
    //
    // Note: MS specification omits the constant 2 in calculations. This library is taking a better approach...
    //
    // sectors_per_fat >= ((t0 + 2 * sectors_per_cluster) / sectors_per_cluster) / (bits_per_sector / bits_per_fat_entry + fats / sectors_per_cluster)
    // sectors_per_fat >= (t0 + 2 * sectors_per_cluster) / (sectors_per_cluster * bits_per_sector / bits_per_fat_entry + fats)
    //
    // Note: compared to MS formula this one can suffer from an overflow problem if u32 type is used
    //
    // When converting formula to integer types round towards a bigger FAT:
    // * first division towards infinity
    // * second division towards zero (it is in a denominator of the first division)

    let t0: u32 = total_sectors - u32::from(reserved_sectors) - root_dir_sectors;
    let t1: u64 = u64::from(t0) + u64::from(2 * u32::from(sectors_per_cluster));
    let bits_per_cluster =
        u32::from(sectors_per_cluster) * u32::from(bytes_per_sector) * BITS_PER_BYTE;
    let t2 = u64::from(bits_per_cluster / fat_type.bits_per_fat_entry() + u32::from(fats));
    let sectors_per_fat = (t1 + t2 - 1) / t2;
    // Note: casting is safe here because number of sectors per FAT cannot be bigger than total sectors number
    sectors_per_fat as u32
}

fn try_fs_geometry(
    total_sectors: u32,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    fat_type: FatType,
    root_dir_sectors: u32,
    fats: u8,
) -> Result<(u16, u32), Error<()>> {
    // Note: most of implementations use 32 reserved sectors for FAT32 but it's wasting of space
    // This implementation uses only 8. This is enough to fit in two boot sectors (main and backup) with additional
    // bootstrap code and one FSInfo sector. It also makes FAT alligned to 4096 which is a nice number.
    let reserved_sectors: u16 = if fat_type == FatType::Fat32 { 8 } else { 1 };

    // Check if volume has enough space to accomodate reserved sectors, FAT, root directory and some data space
    // Having less than 8 sectors for FAT and data would make a little sense
    if total_sectors <= u32::from(reserved_sectors) + root_dir_sectors + 8 {
        panic!("Volume is too small");
        return Err(Error::InvalidInput);
    }

    // calculate File Allocation Table size
    let sectors_per_fat = determine_sectors_per_fat(
        total_sectors,
        bytes_per_sector,
        sectors_per_cluster,
        fat_type,
        reserved_sectors,
        root_dir_sectors,
        fats,
    );

    let data_sectors = total_sectors
        - u32::from(reserved_sectors)
        - root_dir_sectors
        - sectors_per_fat * u32::from(fats);
    let total_clusters = data_sectors / u32::from(sectors_per_cluster);
    // if fat_type != FatType::from_clusters(total_clusters) {
    //     panic!("Invalid FAT type");
    //     return Err(Error::InvalidInput);
    // }
    debug_assert!(total_clusters >= fat_type.min_clusters());
    if total_clusters > fat_type.max_clusters() {
        // Note: it can happen for FAT32
        panic!("Too many clusters");
        return Err(Error::InvalidInput);
    }

    Ok((reserved_sectors, sectors_per_fat))
}

fn determine_root_dir_sectors(
    root_dir_entries: u16,
    bytes_per_sector: u16,
    fat_type: FatType,
) -> u32 {
    if fat_type == FatType::Fat32 {
        0
    } else {
        let root_dir_bytes = u32::from(root_dir_entries) * DIR_ENTRY_SIZE as u32;
        (root_dir_bytes + u32::from(bytes_per_sector) - 1) / u32::from(bytes_per_sector)
    }
}

fn determine_fs_geometry(
    total_sectors: u32,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    root_dir_entries: u16,
    fats: u8,
) -> Result<(FatType, u16, u32), ()> {
    for &fat_type in &[FatType::Fat32, FatType::Fat16, FatType::Fat12] {
        let root_dir_sectors =
            determine_root_dir_sectors(root_dir_entries, bytes_per_sector, fat_type);
        let result = try_fs_geometry(
            total_sectors,
            bytes_per_sector,
            sectors_per_cluster,
            fat_type,
            root_dir_sectors,
            fats,
        );
        if let Ok((reserved_sectors, sectors_per_fat)) = result {
            return Ok((fat_type, reserved_sectors, sectors_per_fat));
        }
    }

    panic!("Cannot select FAT type - unfortunate storage size");
    Err(Error::InvalidInput)
}

fn format_bpb(
    options: &FormatVolumeOptions,
    total_sectors: u32,
    bytes_per_sector: u16,
) -> Result<(BiosParameterBlock, FatType), ()> {
    let bytes_per_cluster = options.bytes_per_cluster.unwrap_or_else(|| {
        let total_bytes = u64::from(total_sectors) * u64::from(bytes_per_sector);
        determine_bytes_per_cluster(total_bytes, bytes_per_sector, options.fat_type)
    });

    let sectors_per_cluster = bytes_per_cluster / u32::from(bytes_per_sector);
    assert!(sectors_per_cluster <= u32::from(u8::MAX));
    let sectors_per_cluster = sectors_per_cluster as u8;

    let fats = options.fats.unwrap_or(2_u8);
    let root_dir_entries = options.max_root_dir_entries.unwrap_or(512);
    let (fat_type, reserved_sectors, sectors_per_fat) = determine_fs_geometry(
        total_sectors,
        bytes_per_sector,
        sectors_per_cluster,
        root_dir_entries,
        fats,
    )?;

    // drive_num should be 0 for floppy disks and 0x80 for hard disks - determine it using FAT type
    let drive_num =
        options
            .drive_num
            .unwrap_or_else(|| if fat_type == FatType::Fat12 { 0 } else { 0x80 });

    // reserved_0 is always zero
    let reserved_0 = [0_u8; 12];

    // setup volume label
    let mut volume_label = [0_u8; 11];
    if let Some(volume_label_from_opts) = options.volume_label {
        volume_label.copy_from_slice(&volume_label_from_opts);
    } else {
        volume_label.copy_from_slice(b"NO NAME    ");
    }

    // setup fs_type_label field
    let mut fs_type_label = [0_u8; 8];
    let fs_type_label_str = match fat_type {
        FatType::Fat12 => b"FAT12   ",
        FatType::Fat16 => b"FAT16   ",
        FatType::Fat32 => b"FAT32   ",
    };
    fs_type_label.copy_from_slice(fs_type_label_str);

    // create Bios Parameter Block struct
    let is_fat32 = fat_type == FatType::Fat32;
    let sectors_per_fat_16 = if is_fat32 {
        0
    } else {
        debug_assert!(sectors_per_fat <= u32::from(u16::MAX));
        sectors_per_fat as u16
    };
    let bpb = BiosParameterBlock {
        bytes_per_sector,
        sectors_per_cluster,
        reserved_sectors,
        fats,
        root_entries: if is_fat32 { 0 } else { root_dir_entries },
        total_sectors_16: if total_sectors < 0x10000 {
            total_sectors as u16
        } else {
            0
        },
        media: options.media.unwrap_or(0xF8),
        sectors_per_fat_16,
        sectors_per_track: options.sectors_per_track.unwrap_or(0x20),
        // heads: options.heads.unwrap_or(0x40),
        heads: 0x40,
        hidden_sectors: 0,
        total_sectors_32: if total_sectors >= 0x10000 {
            total_sectors
        } else {
            0
        },
        // FAT32 fields start
        sectors_per_fat_32: if is_fat32 { sectors_per_fat } else { 0 },
        extended_flags: 0, // mirroring enabled
        fs_version: 0,
        root_dir_first_cluster: if is_fat32 { 2 } else { 0 },
        fs_info_sector: if is_fat32 { 1 } else { 0 },
        backup_boot_sector: if is_fat32 { 6 } else { 0 },
        reserved_0,
        // FAT32 fields end
        drive_num,
        reserved_1: 0,
        ext_sig: 0x29,
        volume_id: options.volume_id.unwrap_or(0x1234_5678),
        volume_label,
        fs_type_label,
    };

    // Check if number of clusters is proper for used FAT type
    if FatType::from_clusters(bpb.total_clusters()) != fat_type {
        panic!("Total number of clusters and FAT type does not match, please try a different volume size");
        return Err(Error::InvalidInput);
    }

    Ok((bpb, fat_type))
}

pub(crate) fn format_boot_sector(
    options: &FormatVolumeOptions,
    total_sectors: u32,
    bytes_per_sector: u16,
) -> Result<(BootSector, FatType), ()> {
    let mut boot = BootSector::default();
    let (bpb, fat_type) = format_bpb(options, total_sectors, bytes_per_sector)?;
    boot.bpb = bpb;
    boot.oem_name.copy_from_slice(b"MSWIN4.1");
    // Boot code copied from FAT32 boot sector initialized by mkfs.fat
    boot.bootjmp = [0xEB, 0x58, 0x90];
    let boot_code: [u8; 129] = [
        0x0E, 0x1F, 0xBE, 0x77, 0x7C, 0xAC, 0x22, 0xC0, 0x74, 0x0B, 0x56, 0xB4, 0x0E, 0xBB, 0x07,
        0x00, 0xCD, 0x10, 0x5E, 0xEB, 0xF0, 0x32, 0xE4, 0xCD, 0x16, 0xCD, 0x19, 0xEB, 0xFE, 0x54,
        0x68, 0x69, 0x73, 0x20, 0x69, 0x73, 0x20, 0x6E, 0x6F, 0x74, 0x20, 0x61, 0x20, 0x62, 0x6F,
        0x6F, 0x74, 0x61, 0x62, 0x6C, 0x65, 0x20, 0x64, 0x69, 0x73, 0x6B, 0x2E, 0x20, 0x20, 0x50,
        0x6C, 0x65, 0x61, 0x73, 0x65, 0x20, 0x69, 0x6E, 0x73, 0x65, 0x72, 0x74, 0x20, 0x61, 0x20,
        0x62, 0x6F, 0x6F, 0x74, 0x61, 0x62, 0x6C, 0x65, 0x20, 0x66, 0x6C, 0x6F, 0x70, 0x70, 0x79,
        0x20, 0x61, 0x6E, 0x64, 0x0D, 0x0A, 0x70, 0x72, 0x65, 0x73, 0x73, 0x20, 0x61, 0x6E, 0x79,
        0x20, 0x6B, 0x65, 0x79, 0x20, 0x74, 0x6F, 0x20, 0x74, 0x72, 0x79, 0x20, 0x61, 0x67, 0x61,
        0x69, 0x6E, 0x20, 0x2E, 0x2E, 0x2E, 0x20, 0x0D, 0x0A,
    ];
    boot.boot_code[..boot_code.len()].copy_from_slice(&boot_code);
    boot.boot_sig = [0x55, 0xAA];

    // fix offsets in bootjmp and boot code for non-FAT32 filesystems (bootcode is on a different offset)
    if fat_type != FatType::Fat32 {
        // offset of boot code
        const BOOT_CODE_OFFSET: u8 = 0x36 + 8;
        // offset of message
        const MESSAGE_OFFSET: u16 = 29;
        boot.bootjmp[1] = BOOT_CODE_OFFSET - 2;
        let message_offset_in_sector = u16::from(BOOT_CODE_OFFSET) + MESSAGE_OFFSET + 0x7c00;
        boot.boot_code[3] = (message_offset_in_sector & 0xff) as u8;
        boot.boot_code[4] = (message_offset_in_sector >> 8) as u8;
    }

    Ok((boot, fat_type))
}

pub(crate) trait ReadLeExt {
    type Error;
    fn read_u8(&mut self) -> Result<u8, Self::Error>;
    fn read_u16_le(&mut self) -> Result<u16, Self::Error>;
    fn read_u32_le(&mut self) -> Result<u32, Self::Error>;
}

impl<T: Read> ReadLeExt for T {
    type Error = <Self as IoBase>::Error;

    fn read_u8(&mut self) -> Result<u8, Self::Error> {
        let mut buf = [0_u8; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u16_le(&mut self) -> Result<u16, Self::Error> {
        let mut buf = [0_u8; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_u32_le(&mut self) -> Result<u32, Self::Error> {
        let mut buf = [0_u8; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }
}

pub(crate) trait WriteLeExt {
    type Error;
    fn write_u8(&mut self, n: u8) -> Result<(), Self::Error>;
    fn write_u16_le(&mut self, n: u16) -> Result<(), Self::Error>;
    fn write_u32_le(&mut self, n: u32) -> Result<(), Self::Error>;
}

impl<T: Write> WriteLeExt for T {
    type Error = <Self as IoBase>::Error;

    fn write_u8(&mut self, n: u8) -> Result<(), Self::Error> {
        self.write_all(&[n])
    }

    fn write_u16_le(&mut self, n: u16) -> Result<(), Self::Error> {
        self.write_all(&n.to_le_bytes())
    }

    fn write_u32_le(&mut self, n: u32) -> Result<(), Self::Error> {
        self.write_all(&n.to_le_bytes())
    }
}
