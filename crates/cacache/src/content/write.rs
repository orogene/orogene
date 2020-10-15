use std::fs::{DirBuilder, OpenOptions};
use std::io::{Cursor, Seek, Write};
use std::path::{Path, PathBuf};

use memmap::MmapMut;
use ssri::{Algorithm, Integrity, IntegrityOpts};
use tempfile::NamedTempFile;

use crate::content::path;
use crate::errors::{Internal, Result};

pub const MAX_MMAP_WRITE_SIZE: usize = 1024 * 1024 * 10;
#[cfg(not(target_os = "windows"))]
pub const MIN_MMAP_WRITE_SIZE: usize = 1024 * 1024;
#[cfg(target_os = "windows")]
pub const MIN_MMAP_WRITE_SIZE: usize = 1;

pub struct Writer {
    cache: PathBuf,
    builder: IntegrityOpts,
    target: snap::write::FrameEncoder<MaybeCursed>,
    expected_size: Option<usize>,
    written: usize,
}

#[derive(Debug)]
struct MaybeCursed {
    cursor: Option<Cursor<Vec<u8>>>,
    tmpfile: Option<NamedTempFile>,
}

impl Seek for MaybeCursed {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        if let Some(cursor) = self.cursor.as_mut() {
            cursor.seek(pos)
        } else if let Some(tmpfile) = self.tmpfile.as_mut() {
            tmpfile.seek(pos)
        } else {
            unreachable!()
        }
    }
}

impl Write for MaybeCursed {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(cursor) = self.cursor.as_mut() {
            cursor.write(&buf)
        } else if let Some(tmpfile) = self.tmpfile.as_mut() {
            tmpfile.write(&buf)
        } else {
            unreachable!()
        }
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(tmpfile) = self.tmpfile.as_mut() {
            tmpfile.flush()
        } else {
            Ok(())
        }
    }
}

impl Writer {
    pub fn new(cache: PathBuf, algo: Algorithm, size: Option<usize>) -> Result<Self> {
        let cache_path = cache;

        let cursor = size.and_then(|size| {
            if size >= MIN_MMAP_WRITE_SIZE && size <= MAX_MMAP_WRITE_SIZE {
                Some(Cursor::new(Vec::with_capacity(size)))
            } else {
                None
            }
        });

        let tmpfile = if cursor.is_none() {
            let mut tmp_path = cache_path.clone();
            tmp_path.push("tmp");
            DirBuilder::new()
                .recursive(true)
                .create(&tmp_path)
                .to_internal()?;
            Some(NamedTempFile::new_in(tmp_path).to_internal()?)
        } else {
            None
        };

        let mut writer = MaybeCursed { tmpfile, cursor };

        size.and_then(|size| writer.write(&size.to_be_bytes()).ok())
            .or_else(|| writer.write(&0u64.to_be_bytes()).ok());

        Ok(Writer {
            cache: cache_path,
            builder: IntegrityOpts::new().algorithm(algo),
            target: snap::write::FrameEncoder::new(writer),
            expected_size: size,
            written: 0,
        })
    }

    pub fn close(self) -> Result<Integrity> {
        let sri = self.builder.result();
        let cpath = path::content_path(&self.cache, &sri);
        DirBuilder::new()
            .recursive(true)
            // Safe unwrap. cpath always has multiple segments
            .create(cpath.parent().unwrap())
            .to_internal()?;

        let mut maybe_mmap = self.target.into_inner().to_internal()?;

        match self.expected_size {
            None => {
                maybe_mmap.seek(std::io::SeekFrom::Start(0)).to_internal()?;
                let bytes = (self.written as u64).to_be_bytes();

                maybe_mmap.write(&bytes).to_internal()?;
                maybe_mmap.flush().to_internal()?;
            }
            Some(size) => {
                if size != self.written {
                    return Err(crate::errors::Error::SizeError(size, self.written));
                }
            }
        };

        if let Some(tmpfile) = maybe_mmap.tmpfile.take() {
            if tmpfile.persist(&cpath).to_internal().is_err() {
                // We might run into conflicts sometimes when persisting files.
                // This is ok. We can deal. Let's just make sure the destination
                // file actually exists, and we can move on.
                std::fs::metadata(cpath).to_internal()?;
            }
        } else if let Some(cursor) = maybe_mmap.cursor.take() {
            if persist_cursor(cursor, &cpath).is_err() {
                // Same as above
                std::fs::metadata(cpath).to_internal()?;
            }
        }
        Ok(sri)
    }

    pub async fn new_async(
        cache: PathBuf,
        algo: Algorithm,
        size: Option<usize>,
    ) -> Result<smol::Unblock<Self>> {
        smol::unblock!(Writer::new(cache, algo, size)).map(smol::Unblock::new)
    }

    pub async fn close_async(self) -> Result<Integrity> {
        smol::unblock!(self.close())
    }
}

fn persist_cursor(cursor: Cursor<Vec<u8>>, cpath: impl AsRef<Path>) -> Result<()> {
    let buf = cursor.into_inner();
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(cpath.as_ref())
        .to_internal()?;
    file.set_len(buf.len() as u64).to_internal()?;
    let mut mmap = unsafe { MmapMut::map_mut(&file).to_internal()? };
    mmap.copy_from_slice(&buf);
    mmap.flush_async().to_internal()?;
    Ok(())
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.written += buf.len();
        self.builder.input(&buf);
        self.target.write(&buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.target.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Read;

    use pretty_assertions::assert_eq;

    #[test]
    fn basic_write() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_owned();
        let mut writer = Writer::new(dir.clone(), Algorithm::Sha256, None).unwrap();
        writer.write_all(b"hello world").unwrap();
        let sri = writer.close().unwrap();
        assert_eq!(sri.to_string(), Integrity::from(b"hello world").to_string());

        let mut reader = std::fs::File::open(path::content_path(&dir, &sri)).unwrap();
        let mut size_bytes = [0u8; 8];
        reader.read_exact(&mut size_bytes).unwrap();
        let size = u64::from_be_bytes(size_bytes) as usize;
        let mut data = Vec::new();
        snap::read::FrameDecoder::new(reader)
            .read_to_end(&mut data)
            .unwrap();

        // we wrote the correct value.
        assert_eq!(size, 11);

        assert_eq!(data, b"hello world");
    }

    #[test]
    fn sized_write() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_owned();
        let input = b"hello world, how are you";
        let mut writer = Writer::new(dir.clone(), Algorithm::Sha256, Some(input.len())).unwrap();
        writer.write_all(input).unwrap();
        let sri = writer.close().unwrap();
        assert_eq!(sri.to_string(), Integrity::from(input).to_string());

        let mut reader = std::fs::File::open(path::content_path(&dir, &sri)).unwrap();
        let mut size_bytes = [0u8; 8];
        reader.read_exact(&mut size_bytes).unwrap();
        let size = u64::from_be_bytes(size_bytes) as usize;
        let mut data = Vec::new();
        snap::read::FrameDecoder::new(reader)
            .read_to_end(&mut data)
            .unwrap();

        assert_eq!(data.len(), size);

        assert_eq!(data, input);
    }
}
