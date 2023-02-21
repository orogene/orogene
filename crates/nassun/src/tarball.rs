#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

use async_compat::CompatExt;
use async_compression::futures::bufread::GzipDecoder;
use async_tar::Archive;
use futures::StreamExt;
use ssri::{Integrity, IntegrityChecker};
use tempfile::NamedTempFile;
#[cfg(not(target_arch = "wasm32"))]
use tokio::io;
use tokio::io::{AsyncRead, BufReader, BufWriter, ReadBuf};

use crate::entries::{Entries, Entry};
use crate::error::{NassunError, Result};
use crate::TarballStream;

const MAX_IN_MEMORY_TARBALL_SIZE: usize = 1024 * 1024 * 10;

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
        use tokio::io::AsyncReadExt;

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
                'inner: loop {
                    let n = reader.read(&mut buf).await?;
                    if n == 0 {
                        break 'inner;
                    }
                    tempfile.write_all(&buf[..n])?;
                }
                return Ok(TempTarball::File(tempfile));
            }
            vec.extend_from_slice(&buf[..n]);
        }
        Ok(TempTarball::Memory(std::io::Cursor::new(vec)))
    }

    /// A `Stream` of extracted entries from this tarball.
    pub fn entries(self) -> Result<Entries> {
        let decoder = GzipDecoder::new(BufReader::new(self).compat());
        let ar = Archive::new(decoder);
        Ok(Entries(
            ar.clone(),
            Box::new(
                ar.entries()
                    .map_err(|e| NassunError::ExtractIoError(e, None))?
                    .map(|res| {
                        res.map(|e| Entry(e.compat()))
                            .map_err(|e| NassunError::ExtractIoError(e, None))
                    }),
            ),
        ))
    }

    /// Extract this tarball to the given directory.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn extract_to_dir(self, dir: impl AsRef<Path>) -> Result<()> {
        use tokio::io::AsyncReadExt;

        let mut files = self.entries()?;

        let dir = PathBuf::from(dir.as_ref());
        let takeme = dir.clone();
        std::fs::create_dir_all(&takeme)
            .map_err(|e| NassunError::ExtractIoError(e, Some(takeme.clone())))?;

        while let Some(file) = files.next().await {
            let file = file?;
            let header = file.header();
            let entry_path = header
                .path()
                .map_err(|e| NassunError::ExtractIoError(e, None))?;
            let entry_subpath =
                strip_one((&*entry_path).into()).unwrap_or_else(|| entry_path.as_ref().into());
            let path = dir.join(entry_subpath);
            if let async_tar::EntryType::Regular = header.entry_type() {
                let takeme = path.clone();

                std::fs::create_dir_all(takeme.parent().unwrap()).map_err(|e| {
                    NassunError::ExtractIoError(e, Some(takeme.parent().unwrap().into()))
                })?;

                let mut writer = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .map_err(|e| NassunError::ExtractIoError(e, Some(path.clone())))?;

                let size = header
                    .size()
                    .map_err(|e| NassunError::ExtractIoError(e, Some(path.clone())))?;
                if size < 1024 * 1024 {
                    let mut buf = Vec::with_capacity(size as usize);
                    BufReader::new(file)
                        .read_to_end(&mut buf)
                        .await
                        .map_err(|e| NassunError::ExtractIoError(e, Some(path.clone())))?;
                    std::io::BufWriter::new(&mut writer)
                        .write_all(&buf)
                        .map_err(|e| NassunError::ExtractIoError(e, Some(path.clone())))?;
                    writer
                        .flush()
                        .map_err(|e| NassunError::ExtractIoError(e, Some(path.clone())))?;
                } else {
                    let writer = tokio::fs::File::from_std(writer);
                    let mut writer = BufWriter::new(writer);
                    let mut reader = BufReader::new(file);
                    io::copy(&mut reader, &mut writer)
                        .await
                        .map_err(|e| NassunError::ExtractIoError(e, Some(path.clone())))?;
                }
            }
        }
        Ok(())
    }
}

fn strip_one(path: &Path) -> Option<&Path> {
    let mut comps = path.components();
    comps.next().map(|_| comps.as_path())
}

impl AsyncRead for Tarball {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        futures::ready!(Pin::new(&mut self.reader).poll_read(cx, buf))?;
        let filled = buf.filled();
        let mut checker_done = false;
        if let Some(checker) = self.checker.as_mut() {
            if !filled.is_empty() {
                checker.input(filled);
            } else {
                checker_done = true;
            }
        }
        if checker_done
            && self
                .checker
                .take()
                .expect("there should've been a checker here")
                .result()
                .is_err()
        {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Integrity check failed",
            )));
        }
        Poll::Ready(Ok(()))
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
            let gz = flate2::read::GzDecoder::new(reader);
            let mut ar = tar::Archive::new(gz);
            let files = ar.entries()?;

            let dir = PathBuf::from(dir);
            let takeme = dir.clone();
            std::fs::create_dir_all(&takeme)
                .map_err(|e| NassunError::ExtractIoError(e, Some(takeme.clone())))?;

            for file in files {
                let file = file?;
                let header = file.header();
                let entry_path = header
                    .path()
                    .map_err(|e| NassunError::ExtractIoError(e, None))?;
                let entry_subpath = strip_one(&entry_path).unwrap_or_else(|| entry_path.as_ref());
                let path = dir.join(entry_subpath);
                if let tar::EntryType::Regular = header.entry_type() {
                    let takeme = path.clone();

                    std::fs::create_dir_all(takeme.parent().unwrap()).map_err(|e| {
                        NassunError::ExtractIoError(e, Some(takeme.parent().unwrap().into()))
                    })?;

                    let mut writer = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&path)
                        .map_err(|e| NassunError::ExtractIoError(e, Some(path.clone())))
                        .map(std::io::BufWriter::new)?;

                    let mut reader = std::io::BufReader::new(file);

                    std::io::copy(&mut reader, &mut writer)
                        .map_err(|e| NassunError::ExtractIoError(e, Some(path.clone())))?;
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
