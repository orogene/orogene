#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Seek};
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

use async_compression::futures::bufread::GzipDecoder;
use async_std::io::BufReader;
use async_tar_wasm::Archive;
#[cfg(not(target_arch = "wasm32"))]
use backon::{BlockingRetryable, ConstantBuilder};
#[cfg(not(target_arch = "wasm32"))]
use cacache::WriteOpts;
#[cfg(not(target_arch = "wasm32"))]
use futures::AsyncReadExt;
use futures::{AsyncRead, StreamExt};
#[cfg(not(target_arch = "wasm32"))]
use ssri::IntegrityOpts;
use ssri::{Integrity, IntegrityChecker};
#[cfg(not(target_arch = "wasm32"))]
use tempfile::NamedTempFile;

use crate::entries::{Entries, Entry};
use crate::error::{NassunError, Result};
use crate::TarballStream;

#[cfg(not(target_arch = "wasm32"))]
const MAX_IN_MEMORY_TARBALL_SIZE: usize = 1024 * 1024 * 5;

pub struct Tarball {
    checker: Option<IntegrityChecker>,
    reader: TarballStream,
    #[cfg(not(target_arch = "wasm32"))]
    integrity: Option<Integrity>,
}

impl Tarball {
    pub(crate) fn new(reader: TarballStream, integrity: Integrity) -> Self {
        Self {
            reader,
            #[cfg(not(target_arch = "wasm32"))]
            integrity: Some(integrity.clone()),
            checker: Some(IntegrityChecker::new(integrity)),
        }
    }

    pub(crate) fn new_unchecked(reader: TarballStream) -> Self {
        Self {
            reader,
            checker: None,
            #[cfg(not(target_arch = "wasm32"))]
            integrity: None,
        }
    }

    pub fn into_inner(self) -> TarballStream {
        self.reader
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) async fn extract_from_tarball_data(
        mut self,
        dir: &Path,
        cache: Option<&Path>,
        prefer_copy: bool,
    ) -> Result<Integrity> {
        let integrity = self.integrity.take();
        let temp = self.into_temp().await?;
        let dir = PathBuf::from(dir);
        let cache = cache.map(PathBuf::from);
        async_std::task::spawn_blocking(move || {
            temp.extract_to_dir(&dir, integrity, cache.as_deref(), prefer_copy)
        })
        .await
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn into_temp(self) -> Result<TempTarball> {
        let mut reader = BufReader::new(self);
        let mut buf = [0u8; 1024 * 8];
        let mut vec = Vec::new();
        loop {
            let n = reader.read(&mut buf).await.map_err(|e| {
                NassunError::ExtractIoError(e, None, "reading from tarball stream".into())
            })?;
            if n == 0 {
                break;
            }
            if vec.len() + n > MAX_IN_MEMORY_TARBALL_SIZE {
                let mut tempfile = tempfile::NamedTempFile::new().map_err(|e| {
                    NassunError::ExtractIoError(e, None, "creating tarball temp file.".into())
                })?;
                tempfile.write_all(&vec).map_err(|e| {
                    NassunError::ExtractIoError(
                        e,
                        None,
                        "writing tarball contents to temp file".into(),
                    )
                })?;
                tempfile.write_all(&buf[..n]).map_err(|e| {
                    NassunError::ExtractIoError(
                        e,
                        None,
                        "writing tarball contents to temp file".into(),
                    )
                })?;
                loop {
                    let n = reader.read(&mut buf).await.map_err(|e| {
                        NassunError::ExtractIoError(e, None, "reading from tarball stream".into())
                    })?;
                    if n == 0 {
                        return Ok(TempTarball::File(tempfile));
                    }
                    tempfile.write_all(&buf[..n]).map_err(|e| {
                        NassunError::ExtractIoError(
                            e,
                            None,
                            "writing tarball contents to temp file".into(),
                        )
                    })?;
                }
            }
            vec.extend_from_slice(&buf[..n]);
        }
        Ok(TempTarball::Memory(std::io::Cursor::new(vec)))
    }

    /// A `Stream` of extracted entries from this tarball.
    pub(crate) fn entries(self) -> Result<Entries> {
        let decoder = GzipDecoder::new(BufReader::new(self));
        let ar = Archive::new(decoder);
        Ok(Entries(
            ar.clone(),
            Box::new(
                ar.entries()
                    .map_err(|e| {
                        NassunError::ExtractIoError(e, None, "getting archive entries".into())
                    })?
                    .map(|res| {
                        res.map(Entry).map_err(|e| {
                            NassunError::ExtractIoError(
                                e,
                                None,
                                "reading entry from archive.".into(),
                            )
                        })
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
pub(crate) enum TempTarball {
    File(NamedTempFile),
    Memory(std::io::Cursor<Vec<u8>>),
}

#[cfg(not(target_arch = "wasm32"))]
impl TempTarball {
    pub(crate) fn extract_to_dir(
        mut self,
        dir: &Path,
        tarball_integrity: Option<Integrity>,
        cache: Option<&Path>,
        prefer_copy: bool,
    ) -> Result<Integrity> {
        let mut file_index = serde_json::Map::new();
        let mut drain_buf = [0u8; 1024 * 8];

        self.rewind()?;

        let mut reader = std::io::BufReader::new(self);
        let mut integrity = IntegrityOpts::new().algorithm(ssri::Algorithm::Sha512);
        let mut tee_reader = io_tee::TeeReader::new(&mut reader, &mut integrity);
        let gz = std::io::BufReader::new(flate2::read::GzDecoder::new(&mut tee_reader));
        let mut ar = tar::Archive::new(gz);
        let files = ar.entries().map_err(|e| {
            NassunError::ExtractIoError(e, None, "getting tarball entries iterator".into())
        })?;

        std::fs::create_dir_all(dir).map_err(|e| {
            NassunError::ExtractIoError(
                e,
                Some(PathBuf::from(dir)),
                "creating destination directory for tarball.".into(),
            )
        })?;

        for file in files {
            let mut file = file.map_err(|e| {
                NassunError::ExtractIoError(
                    e,
                    Some(PathBuf::from(dir)),
                    "reading entry from tarball".into(),
                )
            })?;
            let header = file.header();
            let entry_path = header.path().map_err(|e| {
                NassunError::ExtractIoError(e, None, "reading path from entry header.".into())
            })?;
            let entry_subpath = strip_one(&entry_path)
                .unwrap_or_else(|| entry_path.as_ref())
                .to_path_buf();
            let path = dir.join(&entry_subpath);
            if let tar::EntryType::Regular = header.entry_type() {
                std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| {
                    NassunError::ExtractIoError(
                        e,
                        Some(path.parent().unwrap().into()),
                        "creating parent directory for entry.".into(),
                    )
                })?;

                if let Some(cache) = cache {
                    let mut writer = WriteOpts::new()
                        .open_hash_sync(cache)
                        .map_err(|e| NassunError::ExtractCacheError(e, Some(path.clone())))?;

                    std::io::copy(&mut file, &mut writer).map_err(|e| {
                        NassunError::ExtractIoError(
                            e,
                            Some(path.clone()),
                            "copying to cacache + node_modules".into(),
                        )
                    })?;

                    let sri = writer
                        .commit()
                        .map_err(|e| NassunError::ExtractCacheError(e, Some(path.clone())))?;

                    extract_from_cache(cache, &sri, &path, prefer_copy, false)?;

                    file_index.insert(
                        entry_subpath.to_string_lossy().into(),
                        sri.to_string().into(),
                    );
                } else {
                    let mut writer = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&path)
                        .map_err(|e| {
                            NassunError::ExtractIoError(
                                e,
                                Some(path.clone()),
                                "Opening destination file inside node_modules.".into(),
                            )
                        })
                        .map(std::io::BufWriter::new)?;

                    std::io::copy(&mut file, &mut writer).map_err(|e| {
                        NassunError::ExtractIoError(
                            e,
                            Some(path),
                            "Copying file to node_modules destination.".into(),
                        )
                    })?;
                }
            } else {
                loop {
                    let n = file.read(&mut drain_buf).map_err(|e| {
                        NassunError::ExtractIoError(e, None, "draining file from tarball.".into())
                    })?;
                    if n == 0 {
                        break;
                    }
                }
            }
        }

        // Drain the rest of the tarball to make sure we have its full
        // contents (there can be trailing data);
        loop {
            let n = tee_reader.read(&mut drain_buf).map_err(|e| {
                NassunError::ExtractIoError(e, None, "flushing out the rest of the tarball".into())
            })?;
            if n == 0 {
                break;
            }
        }

        let integrity = tarball_integrity.unwrap_or_else(|| integrity.result());

        if let Some(cache) = cache {
            cacache::index::insert(
                cache,
                &tarball_key(&integrity),
                WriteOpts::new()
                    // This is just so the index entry is loadable.
                    .integrity("sha256-deadbeef".parse().unwrap())
                    .metadata(file_index.into()),
            )
            .map_err(|e| NassunError::ExtractCacheError(e, None))?;
        }

        Ok(integrity)
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
impl std::io::Seek for TempTarball {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match self {
            TempTarball::File(f) => f.seek(pos),
            TempTarball::Memory(m) => m.seek(pos),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn strip_one(path: &Path) -> Option<&Path> {
    let mut comps = path.components();
    comps.next().map(|_| comps.as_path())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn tarball_key(integrity: &Integrity) -> String {
    format!("nassun::package::{integrity}")
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn extract_from_cache(
    cache: &Path,
    sri: &Integrity,
    to: &Path,
    prefer_copy: bool,
    validate: bool,
) -> Result<()> {
    if prefer_copy {
        copy_from_cache(cache, sri, to, validate)?;
    } else {
        // HACK: This is horrible, but on wsl2 (at least), this
        // was sometimes crashing with an ENOENT (?!), which
        // really REALLY shouldn't happen. So we just retry a few
        // times and hope the problem goes away.
        let op = || hard_link_from_cache(cache, sri, to, validate);
        op.retry(&ConstantBuilder::default().with_delay(Duration::from_millis(50)))
            .notify(|err, wait| {
                tracing::debug!(
                    "Error hard linking from cache: {}. Retrying after {}ms",
                    err,
                    wait.as_micros() / 1000
                )
            })
            .call()
            .or_else(|_| copy_from_cache(cache, sri, to, validate))?;
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn copy_from_cache(cache: &Path, sri: &Integrity, to: &Path, validate: bool) -> Result<()> {
    if validate {
        cacache::copy_hash_sync(cache, sri, to)
            .map_err(|e| NassunError::ExtractCacheError(e, Some(PathBuf::from(to))))?;
    } else {
        cacache::copy_hash_unchecked_sync(cache, sri, to)
            .map_err(|e| NassunError::ExtractCacheError(e, Some(PathBuf::from(to))))?;
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn hard_link_from_cache(cache: &Path, sri: &Integrity, to: &Path, validate: bool) -> Result<()> {
    if validate {
        cacache::hard_link_hash_sync(cache, sri, to)
            .map_err(|e| NassunError::ExtractCacheError(e, Some(PathBuf::from(to))))?;
    } else {
        cacache::hard_link_hash_unchecked_sync(cache, sri, to)
            .map_err(|e| NassunError::ExtractCacheError(e, Some(PathBuf::from(to))))?;
    }
    Ok(())
}
