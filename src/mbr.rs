use anyhow::Context;
use std::{
    fs::{self, File},
    io,
    path::Path,
};
const SECTOR_SIZE: u32 = 512;

pub fn create_mbr_disk(
    bootsector_path: &Path,
    boot_partition_path: &Path,
    out_mbr_path: &Path,
) -> anyhow::Result<()> {
    let mut boot_sector = File::open(bootsector_path).context("failed to open boot sector")?;
    let mut mbr =
        mbrman::MBR::read_from(&mut boot_sector, SECTOR_SIZE).context("failed to read MBR")?;

    for (index, partition) in mbr.iter() {
        if !partition.is_unused() {
            anyhow::bail!("partition {index} should be unused");
        }
    }

    let mut boot_partition =
        File::open(boot_partition_path).context("failed to open FAT boot partition")?;
    let boot_partition_size = boot_partition
        .metadata()
        .context("failed to read file metadata of FAT boot partition")?
        .len();

    mbr[1] = mbrman::MBRPartitionEntry {
        boot: true,
        starting_lba: 1,
        sectors: (boot_partition_size / u64::from(SECTOR_SIZE))
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
        .open(&out_mbr_path)
        .with_context(|| {
            format!(
                "failed to create MBR disk image at `{}`",
                out_mbr_path.display()
            )
        })?;

    mbr.write_into(&mut disk)
        .context("failed to write MBR header to disk image")?;

    io::copy(&mut boot_partition, &mut disk)
        .context("failed to copy FAT image to MBR disk image")?;

    Ok(())
}
