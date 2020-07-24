use std::fs::DirBuilder;
use std::path::PathBuf;
use std::io::Write;

use memmap::MmapMut;
use ssri::{Algorithm, Integrity, IntegrityOpts};
use tempfile::NamedTempFile;

use crate::content::path;
use crate::errors::{Internal, Result};

pub const MAX_MMAP_SIZE: usize = 1024 * 1024;

pub struct Writer {
    cache: PathBuf,
    builder: IntegrityOpts,
    target: zstd::Encoder<MaybeMmap>
}

struct MaybeMmap {
    mmap: Option<(MmapMut, usize)>,
    tmpfile: NamedTempFile,
}

impl Write for MaybeMmap {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some((mmap, pos)) = self.mmap.as_mut() {
            match (&mut mmap[*pos..]).write(&buf) {
                Ok(written) => {
                    *pos += written;
                    Ok(written)
                },
                Err(e) => Err(e)
            }
        } else {
            self.tmpfile.write(&buf)
        }
    }

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

        let mmap = size.and_then(|size| {
            if size <= MAX_MMAP_SIZE {
                unsafe { MmapMut::map_mut(tmpfile.as_file()).ok() }
            } else {
                None
            }
        }).map(|mmap| (mmap, 0));

        Ok(Writer {
            cache: cache_path,
            builder: IntegrityOpts::new().algorithm(algo),
            target: zstd::Encoder::new(MaybeMmap {
                tmpfile,
                mmap,
            }, 0).to_internal()?
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

        let maybe_mmap = self.target.finish().to_internal()?;
        let res = maybe_mmap.tmpfile.persist(&cpath).to_internal();
        if res.is_err() {
            // We might run into conflicts sometimes when persisting files.
            // This is ok. We can deal. Let's just make sure the destination
            // file actually exists, and we can move on.
            std::fs::metadata(cpath).to_internal()?;
        }
        Ok(sri)
    }

    pub async fn new_async(cache: PathBuf, algo: Algorithm, size: Option<usize>) -> Result<smol::Unblock<Self>> {
        smol::unblock!(Writer::new(cache, algo, size)).map(smol::Unblock::new)
    }

    pub async fn close_async(self) -> Result<Integrity> {
        smol::unblock!(self.close())
    }
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
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
    #[test]
    fn basic_write() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_owned();
        let mut writer = Writer::new(dir.clone(), Algorithm::Sha256, None).unwrap();
        writer.write_all(b"hello world").unwrap();
        let sri = writer.close().unwrap();
        assert_eq!(sri.to_string(), Integrity::from(b"hello world").to_string());
        assert_eq!(
            std::fs::read(path::content_path(&dir, &sri)).unwrap(),
            b"hello world"
        );
    }
}
