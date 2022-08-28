#![no_std]
#![no_main]

use crate::{
    disk::{AlignedBuffer, Read, Seek, SeekFrom},
    protected_mode::{
        copy_to_protected_mode, enter_protected_mode_and_jump_to_stage_3, enter_unreal_mode,
    },
};
use bootloader_x86_64_bios_common::{BiosInfo, FramebufferInfo, Region};
use byteorder::{ByteOrder, LittleEndian};
use core::{fmt::Write as _, slice};
use disk::AlignedArrayBuffer;
use mbr_nostd::{PartitionTableEntry, PartitionType};

mod dap;
mod disk;
mod fat;
mod memory_map;
mod protected_mode;
mod screen;
mod vesa;

/// We use this partition type to store the second bootloader stage;
const BOOTLOADER_SECOND_STAGE_PARTITION_TYPE: u8 = 0x20;

const STAGE_3_DST: *mut u8 = 0x0010_0000 as *mut u8; // 1MiB (typically 14MiB accessible here)
const STAGE_4_DST: *mut u8 = 0x0020_0000 as *mut u8; // 2MiB (typically still 13MiB accessible here)
const KERNEL_DST: *mut u8 = 0x0100_0000 as *mut u8; // 16MiB

extern "C" {
    static _second_stage_end: u8;
}

fn second_stage_end() -> *const u8 {
    unsafe { &_second_stage_end }
}

static mut DISK_BUFFER: AlignedArrayBuffer<0x4000> = AlignedArrayBuffer {
    buffer: [0; 0x4000],
};

#[no_mangle]
#[link_section = ".start"]
pub extern "C" fn _start(disk_number: u16, partition_table_start: *const u8) -> ! {
    start(disk_number, partition_table_start)
}

fn start(disk_number: u16, partition_table_start: *const u8) -> ! {
    screen::Writer.write_str(" -> SECOND STAGE\n").unwrap();

    enter_unreal_mode();

    // parse partition table
    let partitions = {
        const MAX_ENTRIES: usize = 4;
        const ENTRY_SIZE: usize = 16;

        let mut entries = [PartitionTableEntry::empty(); MAX_ENTRIES];
        let raw = unsafe { slice::from_raw_parts(partition_table_start, ENTRY_SIZE * MAX_ENTRIES) };
        for idx in 0..MAX_ENTRIES {
            let offset = idx * ENTRY_SIZE;
            let partition_type = PartitionType::from_mbr_tag_byte(raw[offset + 4]);
            let lba = LittleEndian::read_u32(&raw[offset + 8..]);
            let len = LittleEndian::read_u32(&raw[offset + 12..]);
            entries[idx] = PartitionTableEntry::new(partition_type, lba, len);
        }
        entries
    };
    // look for second stage partition
    let second_stage_partition_idx = partitions
        .iter()
        .enumerate()
        .find(|(_, e)| {
            e.partition_type == PartitionType::Unknown(BOOTLOADER_SECOND_STAGE_PARTITION_TYPE)
        })
        .unwrap()
        .0;
    let fat_partition = partitions.get(second_stage_partition_idx + 1).unwrap();
    assert!(matches!(
        fat_partition.partition_type,
        PartitionType::Fat12(_) | PartitionType::Fat16(_) | PartitionType::Fat32(_)
    ));

    // load fat partition
    let mut disk = disk::DiskAccess {
        disk_number,
        base_offset: u64::from(fat_partition.logical_block_address) * 512,
        current_offset: 0,
    };

    let mut fs = fat::FileSystem::parse(disk.clone());

    let disk_buffer = unsafe { &mut DISK_BUFFER };

    let stage_3_len = load_file("boot-stage-3", STAGE_3_DST, &mut fs, &mut disk, disk_buffer);
    writeln!(screen::Writer, "stage 3 loaded at {STAGE_3_DST:#p}").unwrap();
    let stage_4_dst = {
        let stage_3_end = STAGE_3_DST.wrapping_add(usize::try_from(stage_3_len).unwrap());
        assert!(STAGE_4_DST > stage_3_end);
        STAGE_4_DST
    };
    let stage_4_len = load_file("boot-stage-4", stage_4_dst, &mut fs, &mut disk, disk_buffer);
    writeln!(screen::Writer, "stage 4 loaded at {stage_4_dst:#p}").unwrap();
    let kernel_len = load_file("kernel-x86_64", KERNEL_DST, &mut fs, &mut disk, disk_buffer);
    writeln!(screen::Writer, "kernel loaded at {KERNEL_DST:#p}").unwrap();

    let memory_map = memory_map::query_memory_map().unwrap();
    writeln!(screen::Writer, "{memory_map:x?}").unwrap();

    let mut vesa_info = vesa::VesaInfo::query(disk_buffer).unwrap();
    let vesa_mode = vesa_info.get_best_mode(1000, 1000).unwrap().unwrap();
    writeln!(
        screen::Writer,
        "VESA MODE: {}x{}",
        vesa_mode.width,
        vesa_mode.height
    )
    .unwrap();
    vesa_mode.enable().unwrap();

    let mut info = BiosInfo {
        stage_4: Region {
            start: stage_4_dst as u64,
            len: stage_4_len,
        },
        kernel: Region {
            start: KERNEL_DST as u64,
            len: kernel_len,
        },
        memory_map_addr: memory_map.as_mut_ptr() as u32,
        memory_map_len: memory_map.len().try_into().unwrap(),
        framebuffer: FramebufferInfo {
            region: Region {
                start: vesa_mode.framebuffer_start.into(),
                len: u64::from(vesa_mode.height) * u64::from(vesa_mode.bytes_per_scanline),
            },
            width: vesa_mode.width,
            height: vesa_mode.height,
            bytes_per_pixel: vesa_mode.bytes_per_pixel,
            stride: vesa_mode.bytes_per_scanline / u16::from(vesa_mode.bytes_per_pixel),
            pixel_format: vesa_mode.pixel_format,
        },
    };

    enter_protected_mode_and_jump_to_stage_3(STAGE_3_DST, &mut info);

    loop {}
}

fn load_file(
    file_name: &str,
    dst: *mut u8,
    fs: &mut fat::FileSystem<disk::DiskAccess>,
    disk: &mut disk::DiskAccess,
    disk_buffer: &mut AlignedArrayBuffer<16384>,
) -> u64 {
    let disk_buffer_size = disk_buffer.buffer.len();
    let file = fs
        .find_file_in_root_dir(file_name, disk_buffer)
        .expect("file not found");
    let file_size = file.file_size().into();

    let mut total_offset = 0;
    for cluster in fs.file_clusters(&file) {
        let cluster = cluster.unwrap();
        let cluster_start = cluster.start_offset;
        let cluster_end = cluster_start + u64::from(cluster.len_bytes);

        let mut offset = 0;
        loop {
            let range_start = cluster_start + offset;
            if range_start >= cluster_end {
                break;
            }
            let range_end = u64::min(
                range_start + u64::try_from(disk_buffer_size).unwrap(),
                cluster_end,
            );
            let len = range_end - range_start;

            disk.seek(SeekFrom::Start(range_start));
            disk.read_exact_into(disk_buffer_size, disk_buffer);

            let slice = &disk_buffer.buffer[..usize::try_from(len).unwrap()];
            unsafe { copy_to_protected_mode(dst.wrapping_add(total_offset), slice) };
            let written =
                unsafe { protected_mode::read_from_protected_mode(dst.wrapping_add(total_offset)) };
            assert_eq!(slice[0], written);

            offset += len;
            total_offset += usize::try_from(len).unwrap();
        }
    }
    file_size
}

/// Taken from https://github.com/rust-lang/rust/blob/e100ec5bc7cd768ec17d75448b29c9ab4a39272b/library/core/src/slice/mod.rs#L1673-L1677
///
/// TODO replace with `split_array` feature in stdlib as soon as it's stabilized,
/// see https://github.com/rust-lang/rust/issues/90091
fn split_array_ref<const N: usize, T>(slice: &[T]) -> (&[T; N], &[T]) {
    if N > slice.len() {
        fail(b'S');
    }
    let (a, b) = slice.split_at(N);
    // SAFETY: a points to [T; N]? Yes it's [T] of length N (checked by split_at)
    unsafe { (&*(a.as_ptr() as *const [T; N]), b) }
}

#[cold]
#[inline(never)]
#[no_mangle]
pub extern "C" fn fail(code: u8) -> ! {
    panic!("fail: {}", code as char);
}
