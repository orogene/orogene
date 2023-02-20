#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

use async_compression::futures::bufread::GzipDecoder;
#[cfg(not(target_arch = "wasm32"))]
use async_std::io;
use async_std::io::{BufReader, BufWriter};
use async_tar::Archive;
use futures::{AsyncRead, AsyncReadExt, StreamExt};
use memmap2::MmapMut;
use ssri::{Integrity, IntegrityChecker};

use crate::entries::{Entries, Entry};
use crate::error::{NassunError, Result};
use crate::TarballStream;

pub const MAX_MMAP_SIZE: u64 = 1024 * 1024;
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

    /// Extract this tarball to the given directory.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn extract_to_dir(self, dir: impl AsRef<Path>) -> Result<()> {
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

                let mut writer = async_std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .await
                    .map_err(|e| NassunError::ExtractIoError(e, Some(path.clone())))?;

                let size = header.size()?;
                writer.set_len(size).await?;

                let mmap = if size <= MAX_MMAP_SIZE {
                    unsafe { MmapMut::map_mut(&writer).ok() }
                } else {
                    None
                };

                if let Some(mut mmap) = mmap {
                    let mut buf = [0u8; 8 * 1024];
                    let mut buf_reader = BufReader::new(file);
                    loop {
                        let bytes = buf_reader.read(&mut buf).await?;
                        if bytes == 0 {
                            break;
                        }
                        mmap.copy_from_slice(&buf[..bytes]);
                    }
                } else {
                    io::copy(BufReader::new(file), BufWriter::new(&mut writer))
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
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let amt = futures::ready!(Pin::new(&mut self.reader).poll_read(cx, buf))?;
        if let Some(checker) = self.checker.as_mut() {
            if amt > 0 {
                checker.input(&buf[..amt]);
            } else {
                let mut final_checker = IntegrityChecker::new(Integrity {
                    hashes: Vec::with_capacity(0),
                });
                std::mem::swap(checker, &mut final_checker);
                if final_checker.result().is_err() {
                    return Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Integrity check failed",
                    )));
                }
            }
        }
        Poll::Ready(Ok(amt))
    }
}
