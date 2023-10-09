#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
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
use oro_common::BuildManifest;
#[cfg(not(target_arch = "wasm32"))]
use ssri::IntegrityOpts;
use ssri::{Integrity, IntegrityChecker};
#[cfg(not(target_arch = "wasm32"))]
use tempfile::NamedTempFile;

use crate::entries::{Entries, Entry};
#[cfg(not(target_arch = "wasm32"))]
use crate::error::IoContext;
use crate::error::{NassunError, Result};
#[cfg(not(target_arch = "wasm32"))]
use crate::package::ExtractMode;
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
        extract_mode: ExtractMode,
    ) -> Result<Integrity> {
        let integrity = self.integrity.take();
        let temp = self.into_temp().await?;
        let dir = PathBuf::from(dir);
        let cache = cache.map(PathBuf::from);
        async_std::task::spawn_blocking(move || {
            temp.extract_to_dir(&dir, integrity, cache.as_deref(), extract_mode)
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
        mut extract_mode: ExtractMode,
    ) -> Result<Integrity> {
        let mut build_mani: Option<BuildManifest> = None;
        let mut tarball_index = TarballIndex::default();
        let mut drain_buf = [0u8; 1024 * 8];
        let created = dashmap::DashSet::new();

        self.rewind().io_context(|| {
            format!(
                "Failed to seek to the beginning of temp tarball fd while extracting to dir: {}",
                dir.display()
            )
        })?;

        let mut reader = std::io::BufReader::new(self);
        let mut integrity = IntegrityOpts::new().algorithm(ssri::Algorithm::Xxh3);
        let mut tee_reader = io_tee::TeeReader::new(&mut reader, &mut integrity);
        let gz = std::io::BufReader::new(flate2::read::GzDecoder::new(&mut tee_reader));
        let mut ar = tar::Archive::new(gz);
        let files = ar.entries().map_err(|e| {
            NassunError::ExtractIoError(e, None, "getting tarball entries iterator".into())
        })?;

        mkdirp(dir, &created)?;

        for file in files {
            let mut file = file.map_err(|e| {
                NassunError::ExtractIoError(
                    e,
                    Some(PathBuf::from(dir)),
                    "reading entry from tarball".into(),
                )
            })?;
            let header = file.header();
            let mode = header.mode().unwrap_or(0o644) | 0o600;
            let entry_path = header.path().map_err(|e| {
                NassunError::ExtractIoError(e, None, "reading path from entry header.".into())
            })?;
            let entry_subpath = strip_one(&entry_path)
                .unwrap_or_else(|| entry_path.as_ref())
                .to_path_buf();
            let path = dir.join(&entry_subpath);
            if let tar::EntryType::Regular = header.entry_type() {
                let parent = path.parent().unwrap();
                mkdirp(parent, &created)?;

                if let Some(cache) = cache {
                    let mut writer = WriteOpts::new()
                        .algorithm(cacache::Algorithm::Xxh3)
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

                    extract_from_cache(cache, &sri, &path, extract_mode, mode)?;

                    let entry_subpath = entry_subpath.to_string_lossy().to_string();

                    // We check whether the package has any install scripts.
                    // If so, we need to re-extract all previous files as full
                    // copies and mark the package as having scripts in it.
                    if entry_subpath == "package.json" {
                        let manifest = BuildManifest::from_path(&path).io_context(|| {
                            format!(
                                "Failed to read BuildManifest from path at {}.",
                                path.display()
                            )
                        })?;
                        if ["preinstall", "install", "postinstall"]
                            .iter()
                            .any(|s| manifest.scripts.contains_key(*s))
                        {
                            tarball_index.should_copy = true;
                            if !extract_mode.is_copy() {
                                extract_mode = ExtractMode::Auto;
                                for (entry, (sri, mode)) in &tarball_index.files {
                                    let path = dir.join(entry);
                                    std::fs::remove_file(&path).io_context(|| format!("Failed to remove target file while extracting a new version, at {}.", path.display()))?;
                                    let sri = sri.parse()?;
                                    extract_from_cache(cache, &sri, &path, extract_mode, *mode)?;
                                }
                            }
                        }
                        build_mani = Some(manifest);
                    }
                    tarball_index
                        .files
                        .insert(entry_subpath, (sri.to_string(), mode));
                } else {
                    let mut open_opts = std::fs::OpenOptions::new();
                    open_opts.write(true).create_new(true);

                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::OpenOptionsExt;
                        open_opts.mode(mode);
                    }

                    let mut writer = open_opts
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
                            Some(path.clone()),
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

        if let Some(BuildManifest { bin, .. }) = &build_mani {
            for binpath in bin.values() {
                tarball_index
                    .bin_paths
                    .push(binpath.to_string_lossy().to_string());
                #[cfg(unix)]
                set_bin_mode(&dir.join(binpath))?;
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
                    .integrity("xxh3-deadbeef".parse().unwrap())
                    .raw_metadata(
                        rkyv::util::to_bytes::<_, 1024>(&tarball_index)
                            .map_err(|e| NassunError::SerializeCacheError(format!("{e}")))?
                            .into_vec(),
                    ),
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
#[derive(rkyv::Archive, rkyv::Serialize, Default)]
#[archive(check_bytes)]
pub(crate) struct TarballIndex {
    pub(crate) should_copy: bool,
    pub(crate) bin_paths: Vec<String>,
    pub(crate) files: HashMap<String, (String, u32)>,
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
    extract_mode: ExtractMode,
    #[allow(unused_variables)] mode: u32,
) -> Result<()> {
    match extract_mode {
        ExtractMode::Auto => {
            reflink_from_cache(cache, sri, to).or_else(|_| copy_from_cache(cache, sri, to))?;
        }
        ExtractMode::AutoHardlink | ExtractMode::Hardlink => {
            // HACK: This is horrible, but on wsl2 (at least), this
            // was sometimes crashing with an ENOENT (?!), which
            // really REALLY shouldn't happen. So we just retry a few
            // times and hope the problem goes away.
            (|| hard_link_from_cache(cache, sri, to))
                .retry(&ConstantBuilder::default().with_delay(Duration::from_millis(50)))
                .notify(|err, wait| {
                    tracing::debug!(
                        "Error hard linking from cache: {}. Retrying after {}ms",
                        err,
                        wait.as_micros() / 1000
                    )
                })
                .call()
                // NOTE: we still want the operation to complete if hard linking fails.
                .or_else(|_| reflink_from_cache(cache, sri, to))
                .or_else(|_| copy_from_cache(cache, sri, to))?;
        }
        ExtractMode::Copy => copy_from_cache(cache, sri, to)?,
        ExtractMode::Reflink => reflink_from_cache(cache, sri, to)?,
    }
    #[cfg(unix)]
    {
        if mode != 0o644 {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(to, std::fs::Permissions::from_mode(mode)).map_err(|e| {
                NassunError::ExtractIoError(
                    e,
                    Some(to.to_path_buf()),
                    "setting permissions on extracted file.".into(),
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn set_bin_mode(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = std::fs::metadata(path).map_err(|e| {
        NassunError::ExtractIoError(
            e,
            Some(path.to_path_buf()),
            "Getting extracted file metadata.".into(),
        )
    })?;
    let mode = metadata.permissions().mode();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode | 0o111)).map_err(|e| {
        NassunError::ExtractIoError(
            e,
            Some(path.to_path_buf()),
            "setting permissions on extracted file.".into(),
        )
    })?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn copy_from_cache(cache: &Path, sri: &Integrity, to: &Path) -> Result<()> {
    cacache::copy_hash_sync(cache, sri, to)
        .map_err(|e| NassunError::ExtractCacheError(e, Some(PathBuf::from(to))))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn reflink_from_cache(cache: &Path, sri: &Integrity, to: &Path) -> Result<()> {
    cacache::reflink_hash_sync(cache, sri, to)
        .map_err(|e| NassunError::ExtractCacheError(e, Some(PathBuf::from(to))))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn hard_link_from_cache(cache: &Path, sri: &Integrity, to: &Path) -> Result<()> {
    cacache::hard_link_hash_sync(cache, sri, to)
        .map_err(|e| NassunError::ExtractCacheError(e, Some(PathBuf::from(to))))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn mkdirp(path: &Path, cache: &dashmap::DashSet<PathBuf>) -> Result<()> {
    if !cache.contains(path) {
        let grandpa_present = if let Some(grandpa) = path.parent() {
            cache.contains(grandpa)
        } else {
            true
        };
        if grandpa_present {
            std::fs::create_dir(path).map_err(|e| {
                NassunError::ExtractIoError(
                    e,
                    Some(path.parent().unwrap().into()),
                    "creating parent directory for entry.".into(),
                )
            })?;
            cache.insert(path.to_path_buf());
        } else {
            std::fs::create_dir_all(path).map_err(|e| {
                NassunError::ExtractIoError(
                    e,
                    Some(path.parent().unwrap().into()),
                    "creating parent directory for entry.".into(),
                )
            })?;
            for path in path.ancestors() {
                cache.insert(path.to_path_buf());
            }
        }
    }
    Ok(())
}
