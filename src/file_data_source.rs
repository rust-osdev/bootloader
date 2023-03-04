use alloc::vec::Vec;
use anyhow::Context;
use core::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::Cursor;
use std::path::PathBuf;
use std::{fs, io};

#[derive(Clone)]
pub enum FileDataSource {
    File(PathBuf),
    Data(Vec<u8>),
}

impl Debug for FileDataSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            FileDataSource::File(file) => {
                f.write_fmt(format_args!("data source: File {}", file.display()))
            }
            FileDataSource::Data(d) => {
                f.write_fmt(format_args!("data source: {} raw bytes ", d.len()))
            }
        }
    }
}

impl FileDataSource {
    pub fn len(&self) -> anyhow::Result<u64> {
        Ok(match self {
            FileDataSource::File(path) => fs::metadata(path)
                .with_context(|| format!("failed to read metadata of file `{}`", path.display()))?
                .len(),
            FileDataSource::Data(v) => v.len() as u64,
        })
    }

    pub fn copy_to(&self, target: &mut dyn io::Write) -> anyhow::Result<()> {
        match self {
            FileDataSource::File(file_path) => {
                io::copy(
                    &mut fs::File::open(file_path).with_context(|| {
                        format!("failed to open `{}` for copying", file_path.display())
                    })?,
                    target,
                )?;
            }
            FileDataSource::Data(contents) => {
                let mut cursor = Cursor::new(contents);
                io::copy(&mut cursor, target)?;
            }
        };

        Ok(())
    }
}
