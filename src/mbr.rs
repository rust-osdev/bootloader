use anyhow::Context;
use mbrman::BOOT_ACTIVE;
use std::{
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};

const SECTOR_SIZE: u32 = 512;

#[cfg(not(feature = "embedded_binaries"))]
pub fn create_mbr_disk(
    bootsector_path: &Path,
    second_stage_path: &Path,
    boot_partition_path: &Path,
    out_mbr_path: &Path,
) -> anyhow::Result<()> {
    let second_stage =
        File::open(second_stage_path).context("failed to open second stage binary")?;
    create_mbr_disk_with_readers(
        File::open(bootsector_path).context("failed to open boot sector")?,
        SecondStageData {
            size: second_stage
                .metadata()
                .context("failed to read file metadata of second stage")?
                .len(),
            reader: second_stage,
        },
        boot_partition_path,
        out_mbr_path,
    )
}

#[cfg(feature = "embedded_binaries")]
pub fn create_mbr_disk(
    bootsector_binary: &[u8],
    second_stage_binary: &[u8],
    boot_partition_path: &Path,
    out_mbr_path: &Path,
) -> anyhow::Result<()> {
    use std::io::Cursor;
    create_mbr_disk_with_readers(
        Cursor::new(bootsector_binary),
        SecondStageData {
            size: second_stage_binary.len() as u64,
            reader: Cursor::new(second_stage_binary),
        },
        boot_partition_path,
        out_mbr_path,
    )
}

struct SecondStageData<R> {
    size: u64,
    reader: R,
}

fn create_mbr_disk_with_readers<R: Read + Seek>(
    bootsector_reader: R,
    second_stage_data: SecondStageData<R>,
    boot_partition_path: &Path,
    out_mbr_path: &Path,
) -> anyhow::Result<()> {
    // let mut boot_sector = File::open(bootsector_path).context("failed to open boot sector")?;
    let mut boot_sector = bootsector_reader;
    let mut mbr =
        mbrman::MBR::read_from(&mut boot_sector, SECTOR_SIZE).context("failed to read MBR")?;

    for (index, partition) in mbr.iter() {
        if !partition.is_unused() {
            anyhow::bail!("partition {index} should be unused");
        }
    }

    let mut second_stage = second_stage_data.reader;
    let second_stage_size = second_stage_data.size;

    let second_stage_start_sector = 1;
    let second_stage_sectors = ((second_stage_size - 1) / u64::from(SECTOR_SIZE) + 1)
        .try_into()
        .context("size of second stage is larger than u32::MAX")?;
    mbr[1] = mbrman::MBRPartitionEntry {
        boot: BOOT_ACTIVE,
        starting_lba: second_stage_start_sector,
        sectors: second_stage_sectors,
        // see BOOTLOADER_SECOND_STAGE_PARTITION_TYPE in `boot_sector` crate
        sys: 0x20,

        first_chs: mbrman::CHS::empty(),
        last_chs: mbrman::CHS::empty(),
    };

    let mut boot_partition =
        File::open(boot_partition_path).context("failed to open FAT boot partition")?;
    let boot_partition_start_sector = second_stage_start_sector + second_stage_sectors;
    let boot_partition_size = boot_partition
        .metadata()
        .context("failed to read file metadata of FAT boot partition")?
        .len();
    mbr[2] = mbrman::MBRPartitionEntry {
        boot: BOOT_ACTIVE,
        starting_lba: boot_partition_start_sector,
        sectors: ((boot_partition_size - 1) / u64::from(SECTOR_SIZE) + 1)
            .try_into()
            .context("size of FAT partition is larger than u32::MAX")?,
        //TODO: is this the correct type?
        sys: 0x0c, // FAT32 with LBA

        first_chs: mbrman::CHS::empty(),
        last_chs: mbrman::CHS::empty(),
    };

    let mut disk = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(out_mbr_path)
        .with_context(|| {
            format!(
                "failed to create MBR disk image at `{}`",
                out_mbr_path.display()
            )
        })?;

    mbr.write_into(&mut disk)
        .context("failed to write MBR header to disk image")?;

    // second stage
    assert_eq!(
        disk.stream_position()
            .context("failed to get disk image seek position")?,
        u64::from(second_stage_start_sector * SECTOR_SIZE)
    );
    io::copy(&mut second_stage, &mut disk)
        .context("failed to copy second stage binary to MBR disk image")?;

    // fat partition
    disk.seek(SeekFrom::Start(
        (boot_partition_start_sector * SECTOR_SIZE).into(),
    ))
    .context("seek failed")?;
    io::copy(&mut boot_partition, &mut disk)
        .context("failed to copy FAT image to MBR disk image")?;

    Ok(())
}
