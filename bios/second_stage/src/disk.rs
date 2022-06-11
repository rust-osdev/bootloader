use crate::{dap, screen, second_stage_end};
use core::{fmt::Write as _, slice};

pub struct DiskAccess {
    pub disk_number: u16,
    pub base_offset: u64,
    pub current_offset: u64,
}

impl fatfs::IoBase for DiskAccess {
    type Error = ();
}

impl fatfs::Read for DiskAccess {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
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
        Ok(buf.len())
    }
}

impl fatfs::Seek for DiskAccess {
    fn seek(&mut self, pos: fatfs::SeekFrom) -> Result<u64, Self::Error> {
        writeln!(screen::Writer, "seek to {pos:?}").unwrap();
        match pos {
            fatfs::SeekFrom::Start(offset) => {
                self.current_offset = offset;
                Ok(self.current_offset)
            }
            fatfs::SeekFrom::Current(offset) => {
                self.current_offset = (i64::try_from(self.current_offset).unwrap() + offset)
                    .try_into()
                    .unwrap();
                Ok(self.current_offset)
            }
            fatfs::SeekFrom::End(_) => Err(()),
        }
    }
}

impl fatfs::Write for DiskAccess {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        unimplemented!()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        unimplemented!()
    }
}
