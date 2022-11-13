use crate::dap;

#[derive(Clone)]
pub struct DiskAccess {
    pub disk_number: u16,
    pub base_offset: u64,
    pub current_offset: u64,
}

impl Read for DiskAccess {
    unsafe fn read_exact(&mut self, len: usize) -> &[u8] {
        let current_sector_offset = usize::try_from(self.current_offset % 512).unwrap();

        static mut TMP_BUF: AlignedArrayBuffer<1024> = AlignedArrayBuffer {
            buffer: [0; 512 * 2],
        };
        let buf = unsafe { &mut TMP_BUF };
        assert!(current_sector_offset + len <= buf.buffer.len());

        self.read_exact_into(buf.buffer.len(), buf);

        &buf.buffer[current_sector_offset..][..len]
    }

    fn read_exact_into(&mut self, len: usize, buf: &mut dyn AlignedBuffer) {
        assert_eq!(len % 512, 0);
        let buf = &mut buf.slice_mut()[..len];

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
            unsafe {
                dap.perform_load(self.disk_number);
            }

            start_lba += u64::from(sectors);
            number_of_sectors -= u64::from(sectors);
            target_addr += u32::from(sectors) * 512;

            if number_of_sectors == 0 {
                break;
            }
        }

        self.current_offset = end_addr;
    }
}

impl Seek for DiskAccess {
    fn seek(&mut self, pos: SeekFrom) -> u64 {
        match pos {
            SeekFrom::Start(offset) => {
                self.current_offset = offset;
                self.current_offset
            }
        }
    }
}

pub trait Read {
    unsafe fn read_exact(&mut self, len: usize) -> &[u8];
    fn read_exact_into(&mut self, len: usize, buf: &mut dyn AlignedBuffer);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    Start(u64),
}

pub trait Seek {
    fn seek(&mut self, pos: SeekFrom) -> u64;
}

#[repr(align(2))]
pub struct AlignedArrayBuffer<const LEN: usize> {
    pub buffer: [u8; LEN],
}

pub trait AlignedBuffer {
    fn slice(&self) -> &[u8];
    fn slice_mut(&mut self) -> &mut [u8];
}

impl<const LEN: usize> AlignedBuffer for AlignedArrayBuffer<LEN> {
    fn slice(&self) -> &[u8] {
        &self.buffer[..]
    }
    fn slice_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[..]
    }
}
