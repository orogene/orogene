#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

use async_compression::futures::bufread::GzipDecoder;
use async_std::io::BufReader;
use async_tar_wasm::Archive;
use futures::{AsyncRead, AsyncReadExt, StreamExt};
use ssri::{Integrity, IntegrityChecker};
use tempfile::NamedTempFile;

use crate::entries::{Entries, Entry};
use crate::error::{NassunError, Result};
use crate::TarballStream;

const MAX_IN_MEMORY_TARBALL_SIZE: usize = 1024 * 1024 * 5;

pub struct Tarball {
    checker: Option<IntegrityChecker>,
    reader: TarballStream,
}

impl Tarball {
    pub(crate) fn new(reader: TarballStream, integrity: Integrity) -> Self {
        Self {
            reader,
            checker: Some(IntegrityChecker::new(integrity)),
        }
    }

    pub(crate) fn new_unchecked(reader: TarballStream) -> Self {
        Self {
            reader,
            checker: None,
        }
    }

    pub fn into_inner(self) -> TarballStream {
        self.reader
    }

    /// Returns a temporarily downloaded version of the tarball. If the
    /// tarball is small, it will be loaded into memory, otherwise it will be
    /// written to a temporary file that will be deleted when the
    /// [`TempTarball`] is dropped.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn to_temp(self) -> Result<TempTarball> {
        let mut reader = BufReader::new(self);
        let mut buf = [0u8; 1024 * 8];
        let mut vec = Vec::new();
        loop {
            let n = reader.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            if vec.len() + n > MAX_IN_MEMORY_TARBALL_SIZE {
                let mut tempfile = tempfile::NamedTempFile::new()?;
                tempfile.write_all(&vec)?;
                tempfile.write_all(&buf[..n])?;
                loop {
                    let n = reader.read(&mut buf).await?;
                    if n == 0 {
                        return Ok(TempTarball::File(tempfile));
                    }
                    tempfile.write_all(&buf[..n])?;
                }
            }
            vec.extend_from_slice(&buf[..n]);
        }
        Ok(TempTarball::Memory(std::io::Cursor::new(vec)))
    }

    /// A `Stream` of extracted entries from this tarball.
    pub fn entries(self) -> Result<Entries> {
        let decoder = GzipDecoder::new(BufReader::new(self));
        let ar = Archive::new(decoder);
        Ok(Entries(
            ar.clone(),
            Box::new(
                ar.entries()
                    .map_err(|e| NassunError::ExtractIoError(e, None))?
                    .map(|res| {
                        res.map(Entry)
                            .map_err(|e| NassunError::ExtractIoError(e, None))
                    }),
            ),
        ))
    }
}

impl AsyncRead for Tarball {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let amt = futures::ready!(Pin::new(&mut self.reader).poll_read(cx, buf))?;
        let mut checker_done = false;
        if let Some(checker) = self.checker.as_mut() {
            if amt > 0 {
                checker.input(&buf[..amt]);
            } else {
                checker_done = true;
            }
        }
        if checker_done
            && self
                .checker
                .take()
                .expect("There should've been a checker here")
                .result()
                .is_err()
        {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Integrity check failed",
            )));
        }
        Poll::Ready(Ok(amt))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub enum TempTarball {
    File(NamedTempFile),
    Memory(std::io::Cursor<Vec<u8>>),
}

#[cfg(not(target_arch = "wasm32"))]
impl TempTarball {
    pub fn extract_to_dir(self, dir: impl AsRef<Path>) -> Result<()> {
        fn inner(me: TempTarball, dir: &Path) -> Result<()> {
            let reader = std::io::BufReader::new(me);
            let gz = std::io::BufReader::new(flate2::read::GzDecoder::new(reader));
            let mut ar = tar::Archive::new(gz);
            let files = ar.entries()?;

            std::fs::create_dir_all(dir)
                .map_err(|e| NassunError::ExtractIoError(e, Some(PathBuf::from(dir))))?;

            for file in files {
                let file = file?;
                let header = file.header();
                let entry_path = header
                    .path()
                    .map_err(|e| NassunError::ExtractIoError(e, None))?;
                let entry_subpath = strip_one(&entry_path).unwrap_or_else(|| entry_path.as_ref());
                let path = dir.join(entry_subpath);
                if let tar::EntryType::Regular = header.entry_type() {
                    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| {
                        NassunError::ExtractIoError(e, Some(path.parent().unwrap().into()))
                    })?;

                    let mut writer = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&path)
                        .map_err(|e| NassunError::ExtractIoError(e, Some(path.clone())))
                        .map(std::io::BufWriter::new)?;

                    let mut reader = std::io::BufReader::new(file);

                    std::io::copy(&mut reader, &mut writer)
                        .map_err(|e| NassunError::ExtractIoError(e, Some(path)))?;
                }
            }
            Ok(())
        }
        inner(self, dir.as_ref())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl std::io::Read for TempTarball {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            TempTarball::File(f) => f.read(buf),
            TempTarball::Memory(m) => m.read(buf),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn strip_one(path: &Path) -> Option<&Path> {
    let mut comps = path.components();
    comps.next().map(|_| comps.as_path())
}
