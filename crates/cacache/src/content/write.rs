use std::fs::DirBuilder;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Mutex;

use async_std::fs as afs;
use async_std::future::Future;
use async_std::task::{self, Context, JoinHandle, Poll};
use futures::io::AsyncWrite;
use futures::prelude::*;
use memmap::MmapMut;
use ssri::{Algorithm, Integrity, IntegrityOpts};
use tempfile::NamedTempFile;

use crate::content::path;
use crate::errors::{Internal, Result};

pub const MAX_MMAP_SIZE: usize = 1024 * 1024;

pub struct Writer {
    cache: PathBuf,
    builder: IntegrityOpts,
    mmap: Option<MmapMut>,
    tmpfile: NamedTempFile,
}

impl Writer {
    pub fn new(cache: &Path, algo: Algorithm, size: Option<usize>) -> Result<Self> {
        let cache_path = cache.to_path_buf();
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
        });

        Ok(Writer {
            cache: cache_path,
            builder: IntegrityOpts::new().algorithm(algo),
            tmpfile,
            mmap,
        })
    }

    pub async fn new_async(cache: &Path, algo: Algorithm, size: Option<usize>) -> Result<smol::Unblock<Self>> {
        let cache_path = cache.to_path_buf();
        let mut tmp_path = cache_path.clone();
        tmp_path.push("tmp");
        afs::DirBuilder::new()
            .recursive(true)
            .create(&tmp_path)
            .await
            .to_internal()?;

        let tmpfile = task::spawn_blocking(|| NamedTempFile::new_in(tmp_path))
            .await
            .to_internal()?;

        let mmap = size.and_then(|size| {
            if size <= MAX_MMAP_SIZE {
                unsafe { MmapMut::map_mut(tmpfile.as_file()).ok() }
            } else {
                None
            }
        });

        Ok(smol::Unblock::new(Writer {
            cache: cache_path,
            builder: IntegrityOpts::new().algorithm(algo),
            tmpfile,
            mmap,
        }))
    }

    pub fn close(self) -> Result<Integrity> {
        let sri = self.builder.result();
        let cpath = path::content_path(&self.cache, &sri);
        DirBuilder::new()
            .recursive(true)
            // Safe unwrap. cpath always has multiple segments
            .create(cpath.parent().unwrap())
            .to_internal()?;
        let res = self.tmpfile.persist(&cpath).to_internal();
        if res.is_err() {
            // We might run into conflicts sometimes when persisting files.
            // This is ok. We can deal. Let's just make sure the destination
            // file actually exists, and we can move on.
            std::fs::metadata(cpath).to_internal()?;
        }
        Ok(sri)
    }

    pub async fn close_async(self) -> Result<Integrity> {
        smol::unblock!(self.close())
    }
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.builder.input(&buf);
        if let Some(mmap) = &mut self.mmap {
            mmap.copy_from_slice(&buf);
            Ok(buf.len())
        } else {
            self.tmpfile.write(&buf)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.tmpfile.flush()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use async_std::task;
    #[test]
    fn basic_write() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_owned();
        let mut writer = Writer::new(&dir, Algorithm::Sha256, None).unwrap();
        writer.write_all(b"hello world").unwrap();
        let sri = writer.close().unwrap();
        assert_eq!(sri.to_string(), Integrity::from(b"hello world").to_string());
        assert_eq!(
            std::fs::read(path::content_path(&dir, &sri)).unwrap(),
            b"hello world"
        );
    }
}
