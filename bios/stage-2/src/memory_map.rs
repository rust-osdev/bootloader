// From http://wiki.osdev.org/Detecting_Memory_(x86)#Getting_an_E820_Memory_Map

use crate::split_array_ref;
use bootloader_x86_64_bios_common::{racy_cell::RacyCell, E820MemoryRegion};
use core::arch::asm;

static MEMORY_MAP: RacyCell<[E820MemoryRegion; 100]> = RacyCell::new(
    [E820MemoryRegion {
        start_addr: 0,
        len: 0,
        region_type: 0,
        acpi_extended_attributes: 0,
    }; 100],
);

/// use the INT 0x15, eax= 0xE820 BIOS function to get a memory map
pub unsafe fn query_memory_map() -> Result<&'static mut [E820MemoryRegion], ()> {
    const SMAP: u32 = 0x534D4150;

    let memory_map = unsafe { MEMORY_MAP.get_mut() };

    let mut i = 0;

    let mut offset = 0;
    let buf = [0u8; 24];
    loop {
        let ret: u32;
        let buf_written_len;
        unsafe {
            asm!(
                "push ebx",
                "mov ebx, edx",
                "mov edx, 0x534D4150",
                "int 0x15",
                "mov edx, ebx",
                "pop ebx",
                inout("eax") 0xe820 => ret,
                inout("edx") offset,
                inout("ecx") buf.len() => buf_written_len,
                in("di") &buf
            )
        };
        if ret != SMAP {
            return Err(());
        }

        if buf_written_len != 0 {
            let buf = &buf[..buf_written_len];

            let (&base_raw, rest) = split_array_ref(buf);
            let (&len_raw, rest) = split_array_ref(rest);
            let (&kind_raw, rest) = split_array_ref(rest);
            let acpi_extended_raw: [u8; 4] = rest.try_into().unwrap_or_default();

            let len = u64::from_ne_bytes(len_raw);
            if len != 0 {
                memory_map[i] = E820MemoryRegion {
                    start_addr: u64::from_ne_bytes(base_raw),
                    len,
                    region_type: u32::from_ne_bytes(kind_raw),
                    acpi_extended_attributes: u32::from_ne_bytes(acpi_extended_raw),
                };
                i += 1;
            }
        }

        if offset == 0 {
            break;
        }
    }

    Ok(&mut memory_map[..i])
}
