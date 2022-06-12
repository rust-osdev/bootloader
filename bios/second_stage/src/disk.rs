use crate::{dap, screen, second_stage_end};
use core::{fmt::Write as _, slice};

#[derive(Clone)]
pub struct DiskAccess {
    pub disk_number: u16,
    pub base_offset: u64,
    pub current_offset: u64,
}

impl Read for DiskAccess {
    fn read_exact(&mut self, buf: &mut [u8]) {
        writeln!(screen::Writer, "read {} bytes", buf.len()).unwrap();

        let end_addr = self.base_offset + self.current_offset + u64::try_from(buf.len()).unwrap();
        let start_lba = (self.base_offset + self.current_offset) / 512;
        let end_lba = (end_addr - 1) / 512;

        let target_addr = u16::try_from(second_stage_end() as usize).unwrap();
        let dap = dap::DiskAddressPacket::from_lba(
            target_addr,
            start_lba,
            u16::try_from(end_lba + 1 - start_lba).unwrap(),
        );
        writeln!(screen::Writer, "dap: {dap:?}").unwrap();
        unsafe {
            dap.perform_load(self.disk_number);
        }

        let data = unsafe { slice::from_raw_parts(target_addr as *const u8, buf.len()) };
        buf.copy_from_slice(data);

        self.current_offset = end_addr;
    }
}

impl Seek for DiskAccess {
    fn seek(&mut self, pos: SeekFrom) -> u64 {
        writeln!(screen::Writer, "seek to {pos:?}").unwrap();
        match pos {
            SeekFrom::Start(offset) => {
                self.current_offset = offset;
                self.current_offset
            }
            SeekFrom::Current(offset) => {
                self.current_offset = (i64::try_from(self.current_offset).unwrap() + offset)
                    .try_into()
                    .unwrap();
                self.current_offset
            }
        }
    }
}

pub trait Read {
    fn read_exact(&mut self, buf: &mut [u8]);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    Start(u64),
    Current(i64),
}

pub trait Seek {
    fn seek(&mut self, pos: SeekFrom) -> u64;
}
