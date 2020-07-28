use std::fs::DirBuilder;
use std::io::Seek;
use std::io::Write;
use std::path::PathBuf;

use memmap::MmapMut;
use ssri::{Algorithm, Integrity, IntegrityOpts};
use tempfile::NamedTempFile;

use crate::content::path;
use crate::errors::{Internal, Result};

pub const MAX_MMAP_WRITE_SIZE: usize = 1024 * 1024 * 10;
pub const MIN_MMAP_WRITE_SIZE: usize = 1024 * 1024;

pub struct Writer {
    cache: PathBuf,
    builder: IntegrityOpts,
    target: snap::write::FrameEncoder<MaybeMmap>,
    expected_size: Option<usize>,
    written: usize,
}

#[derive(Debug)]
struct MaybeMmap {
    mmap: Option<(MmapMut, usize)>,
    tmpfile: NamedTempFile,
}

impl Seek for MaybeMmap {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        if let Some((_, position)) = self.mmap.as_mut() {
            match pos {
                std::io::SeekFrom::Start(xs) => {
                    *position = xs as usize;
                    Ok(xs)
                }
                _ => unimplemented!(),
            }
        } else {
            self.tmpfile.seek(pos)
        }
    }
}

impl Write for MaybeMmap {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some((mmap, pos)) = self.mmap.as_mut() {
            match (&mut mmap[*pos..]).write(&buf) {
                Ok(written) => {
                    *pos += written;
                    Ok(written)
                }
                Err(e) => Err(e),
            }
        } else {
            self.tmpfile.write(&buf)
        }
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        if let Some((mmap, _)) = self.mmap.as_mut() {
            mmap.flush_async()?;
        }

        self.tmpfile.flush()
    }
}

impl Writer {
    pub fn new(cache: PathBuf, algo: Algorithm, size: Option<usize>) -> Result<Self> {
        let cache_path = cache;
        let mut tmp_path = cache_path.clone();
        tmp_path.push("tmp");
        DirBuilder::new()
            .recursive(true)
            .create(&tmp_path)
            .to_internal()?;
        let tmpfile = NamedTempFile::new_in(tmp_path).to_internal()?;

        let mmap = size
            .and_then(|size| {
                if size >= MIN_MMAP_WRITE_SIZE && size <= MAX_MMAP_WRITE_SIZE {
                    unsafe { MmapMut::map_mut(tmpfile.as_file()).ok() }
                } else {
                    None
                }
            })
            .map(|mmap| (mmap, 0));

        let mut writer = MaybeMmap { tmpfile, mmap };

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

        let res = maybe_mmap.tmpfile.persist(&cpath).to_internal();
        if res.is_err() {
            // We might run into conflicts sometimes when persisting files.
            // This is ok. We can deal. Let's just make sure the destination
            // file actually exists, and we can move on.
            std::fs::metadata(cpath).to_internal()?;
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
