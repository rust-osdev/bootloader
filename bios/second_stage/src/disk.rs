use crate::{dap, screen};
use core::fmt::Write as _;

#[derive(Clone)]
pub struct DiskAccess {
    pub disk_number: u16,
    pub base_offset: u64,
    pub current_offset: u64,
}

impl Read for DiskAccess {
    fn read_exact(&mut self, input_buf: &mut [u8]) {
        writeln!(screen::Writer, "read {} bytes", input_buf.len()).unwrap();
        static mut TMP_BUF: [u8; 512] = [0; 512];
        let tmp_buf = unsafe { &mut TMP_BUF[..] };
        let (buf, copy_needed) = if input_buf.len() >= tmp_buf.len() {
            (&mut input_buf[..], false)
        } else {
            (&mut tmp_buf[..], true)
        };
        assert_eq!(buf.len() % 512, 0);

        let end_addr = self.base_offset + self.current_offset + u64::try_from(buf.len()).unwrap();
        let mut start_lba = (self.base_offset + self.current_offset) / 512;
        let end_lba = (end_addr - 1) / 512;

        let mut number_of_sectors = end_lba + 1 - start_lba;
        let mut target_addr = buf.as_ptr_range().start as u32;

        loop {
            let sectors = u64::min(number_of_sectors, 32) as u16;
            let dap = dap::DiskAddressPacket::from_lba(
                start_lba,
                sectors,
                (target_addr & 0b1111) as u16,
                (target_addr >> 4).try_into().unwrap(),
            );
            writeln!(screen::Writer, "dap: {dap:?}").unwrap();
            unsafe {
                dap.perform_load(self.disk_number);
            }

            start_lba += u64::from(sectors);
            number_of_sectors -= u64::from(sectors);
            target_addr = target_addr + u32::from(sectors) * 512;

            if number_of_sectors == 0 {
                break;
            }
        }

        self.current_offset = end_addr;

        if copy_needed {
            let len = input_buf.len();
            for i in 0..len {
                input_buf[i] = tmp_buf[i];
            }
        }

        writeln!(screen::Writer, "read {} bytes done", input_buf.len()).unwrap();
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
