#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

use async_compression::futures::bufread::GzipDecoder;
#[cfg(not(target_arch = "wasm32"))]
use async_std::io;
use async_std::io::BufReader;
use async_tar::Archive;
use futures::prelude::*;
use ssri::{Integrity, IntegrityChecker};

use crate::entries::{Entries, Entry};
use crate::error::{NassunError, Result};
use crate::TarballStream;

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
        mkdirp::mkdirp(&takeme)
            .map_err(|e| NassunError::ExtractIoError(e, Some(takeme.clone())))?;

        while let Some(file) = files.next().await {
            let file = file?;
            let header = file.header();
            let path = dir.join(
                header
                    .path()
                    .map_err(|e| NassunError::ExtractIoError(e, None))?
                    .as_ref(),
            );
            if let async_tar::EntryType::Regular = header.entry_type() {
                let takeme = path.clone();

                mkdirp::mkdirp(takeme.parent().unwrap()).map_err(|e| {
                    NassunError::ExtractIoError(e, Some(takeme.parent().unwrap().into()))
                })?;

                let mut writer = async_std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(&path)
                    .await
                    .map_err(|e| NassunError::ExtractIoError(e, Some(path.clone())))?;

                io::copy(file, async_std::io::BufWriter::new(&mut writer))
                    .await
                    .map_err(|e| NassunError::ExtractIoError(e, Some(path.clone())))?;
            }
        }

        // NOTE: Because we might be caching the tarball itself (or at least
        // generating an `Integrity` for it), we make sure to read to the very
        // end of the tarball stream.

        // NOTE: We probably don't need to do this here, but I want to keep
        // this code as reference for when it's actually needed. Most likely,
        // that will be when we're calculating the `Integrity` of the tarball
        // itself.

        // let mut reader = files
        //     .into_inner()
        //     .into_inner()
        //     .map_err(|_| NassunError::MiscError("Failed to get inner Read".into()))?
        //     .into_inner()
        //     .into_inner();
        // let mut buf = [0u8; 1024];
        // loop {
        //     let n = reader
        //         .read(&mut buf)
        //         .await
        //         .map_err(|e| NassunError::ExtractIoError(e, None))?;
        //     if n > 0 {
        //         continue;
        //     } else {
        //         break;
        //     }
        // }
        Ok(())
    }
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
