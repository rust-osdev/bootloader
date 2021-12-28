// based on https://github.com/rafalh/rust-fatfs/

pub(crate) struct BootSector {
    bootjmp: [u8; 3],
    oem_name: [u8; 8],
    pub(crate) bpb: BiosParameterBlock,
    boot_code: [u8; 448],
    boot_sig: [u8; 2],
}

impl BootSector {
    pub(crate) fn deserialize(bytes: &[u8]) -> Self {
        let mut boot = Self::default();
        let (&bootjmp, bytes) = split_array_ref(bytes);
        let (&oem_name, bytes) = split_array_ref(bytes);

        boot.bootjmp = bootjmp;
        boot.oem_name = oem_name;
        boot.bpb = BiosParameterBlock::deserialize(bytes);

        let bytes = if boot.bpb.is_fat32() {
            let (boot_code, bytes): (&[_; 420], _) = split_array_ref(bytes);
            boot.boot_code[0..420].copy_from_slice(&boot_code[..]);
            bytes
        } else {
            let (&boot_code, bytes) = split_array_ref(bytes);
            boot.boot_code = boot_code;
            bytes
        };
        let (&boot_sig, bytes) = split_array_ref(bytes);
        boot.boot_sig = boot_sig;
        boot
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
    pub fn deserialize(bytes: &[u8]) -> Self {
        let (&bytes_per_sector, bytes) = split_array_ref(bytes);
        let (&[sectors_per_cluster], bytes) = split_array_ref(bytes);
        let (&reserved_sectors, bytes) = split_array_ref(bytes);
        let (&[fats], bytes) = split_array_ref(bytes);
        let (&root_entries, bytes) = split_array_ref(bytes);
        let (&total_sectors_16, bytes) = split_array_ref(bytes);
        let (&[media], bytes) = split_array_ref(bytes);
        let (&sectors_per_fat_16, bytes) = split_array_ref(bytes);
        let (&sectors_per_track, bytes) = split_array_ref(bytes);
        let (&heads, bytes) = split_array_ref(bytes);
        let (&hidden_sectors, bytes) = split_array_ref(bytes);
        let (&total_sectors_32, bytes) = split_array_ref(bytes);

        let mut bpb = Self {
            bytes_per_sector: u16::from_le_bytes(bytes_per_sector),
            sectors_per_cluster,
            reserved_sectors: u16::from_le_bytes(reserved_sectors),
            fats,
            root_entries: u16::from_le_bytes(root_entries),
            total_sectors_16: u16::from_le_bytes(total_sectors_16),
            media,
            sectors_per_fat_16: u16::from_le_bytes(sectors_per_fat_16),
            sectors_per_track: u16::from_le_bytes(sectors_per_track),
            heads: u16::from_le_bytes(heads),
            hidden_sectors: u32::from_le_bytes(hidden_sectors),
            total_sectors_32: u32::from_le_bytes(total_sectors_32),
            ..Self::default()
        };

        let (&sectors_per_fat_32, bytes) = split_array_ref(bytes);
        let (&extended_flags, bytes) = split_array_ref(bytes);
        let (&fs_version, bytes) = split_array_ref(bytes);
        let (&root_dir_first_cluster, bytes) = split_array_ref(bytes);
        let (&fs_info_sector, bytes) = split_array_ref(bytes);
        let (&backup_boot_sector, bytes) = split_array_ref(bytes);
        let (&reserved_0, bytes) = split_array_ref(bytes);

        if bpb.is_fat32() {
            bpb.sectors_per_fat_32 = u32::from_le_bytes(sectors_per_fat_32);
            bpb.extended_flags = u16::from_le_bytes(extended_flags);
            bpb.fs_version = u16::from_le_bytes(fs_version);
            bpb.root_dir_first_cluster = u32::from_le_bytes(root_dir_first_cluster);
            bpb.fs_info_sector = u16::from_le_bytes(fs_info_sector);
            bpb.backup_boot_sector = u16::from_le_bytes(backup_boot_sector);
            bpb.reserved_0 = reserved_0;
        }

        let (&[drive_num], bytes) = split_array_ref(bytes);
        let (&[reserved_1], bytes) = split_array_ref(bytes);
        let (&[ext_sig], bytes) = split_array_ref(bytes);
        let (&volume_id, bytes) = split_array_ref(bytes);
        let (&volume_label, bytes) = split_array_ref(bytes);
        let (&fs_type_label, bytes) = split_array_ref(bytes);

        bpb.drive_num = drive_num;
        bpb.reserved_1 = reserved_1;
        bpb.ext_sig = ext_sig; // 0x29
        bpb.volume_id = u32::from_le_bytes(volume_id);
        bpb.volume_label = volume_label;
        bpb.fs_type_label = fs_type_label;

        // when the extended boot signature is anything other than 0x29, the fields are invalid
        if bpb.ext_sig != 0x29 {
            // fields after ext_sig are not used - clean them
            bpb.volume_id = 0;
            bpb.volume_label = [0; 11];
            bpb.fs_type_label = [0; 8];
        }

        bpb
    }

    pub(crate) fn is_fat32(&self) -> bool {
        // because this field must be zero on FAT32, and
        // because it must be non-zero on FAT12/FAT16,
        // this provides a simple way to detect FAT32
        self.sectors_per_fat_16 == 0
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
