use std::fmt;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::path::PathBuf;

use async_std::sync::Arc;
use oro_common::{CorgiPackument, CorgiVersionMetadata, Packument, VersionMetadata};
use oro_package_spec::PackageSpec;
use ssri::Integrity;

use crate::entries::Entries;
#[cfg(unix)]
use crate::error::IoContext;
#[cfg(not(target_arch = "wasm32"))]
use crate::error::NassunError;
use crate::error::Result;
use crate::fetch::PackageFetcher;
use crate::resolver::PackageResolution;
use crate::tarball::Tarball;
#[cfg(not(target_arch = "wasm32"))]
use crate::tarball::TarballIndex;

/// A resolved package. A concrete version has been determined from its
/// PackageSpec by the version resolver.
#[derive(Clone)]
pub struct Package {
    pub(crate) from: PackageSpec,
    pub(crate) name: String,
    pub(crate) resolved: PackageResolution,
    pub(crate) fetcher: Arc<dyn PackageFetcher>,
    pub(crate) base_dir: PathBuf,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(crate) cache: Arc<Option<PathBuf>>,
}

impl Package {
    /// Original package spec that this `Package` was resolved from.
    pub fn from(&self) -> &PackageSpec {
        &self.from
    }

    /// Name of the package, as it should be used in the dependency graph.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The [`PackageResolution`] that this `Package` was created from.
    pub fn resolved(&self) -> &PackageResolution {
        &self.resolved
    }

    /// The full [`Packument`] that this `Package` was resolved from.
    pub async fn packument(&self) -> Result<Arc<Packument>> {
        self.fetcher.packument(&self.from, &self.base_dir).await
    }

    /// The [`VersionMetadata`], aka the manifest, aka roughly the metadata
    /// defined in `package.json`.
    pub async fn metadata(&self) -> Result<VersionMetadata> {
        self.fetcher.metadata(self).await
    }

    /// The partial (corgi) version of the [`Packument`] that this `Package`
    /// was resolved from.
    pub async fn corgi_packument(&self) -> Result<Arc<CorgiPackument>> {
        self.fetcher
            .corgi_packument(&self.from, &self.base_dir)
            .await
    }

    /// The partial (corgi) version of the [`VersionMetadata`], aka the
    /// manifest, aka roughly the metadata defined in `package.json`.
    pub async fn corgi_metadata(&self) -> Result<CorgiVersionMetadata> {
        self.fetcher.corgi_metadata(self).await
    }

    /// `AsyncRead` of the raw tarball data for this package. The data will
    /// not be checked for integrity based on the current `Package`'s
    /// [`Integrity`]. That is, bad or incomplete data may be returned.
    pub async fn tarball_unchecked(&self) -> Result<Tarball> {
        let data = self.fetcher.tarball(self).await?;
        Ok(Tarball::new_unchecked(data))
    }

    /// `AsyncRead` of the raw tarball data for this package. The data will
    /// be checked for integrity based on the current `Package`'s
    /// [`Integrity`], if present in its [`Package::metadata`]. An
    /// [`std::io::Error`] with [`std::io::ErrorKind::InvalidData`] will be
    /// returned in case of integrity validation failure.
    pub async fn tarball(&self) -> Result<Tarball> {
        let data = self.fetcher.tarball(self).await?;
        if let Some(integrity) = self.resolved.integrity() {
            Ok(Tarball::new(data, integrity.clone()))
        } else {
            self.tarball_unchecked().await
        }
    }

    /// `AsyncRead` of the raw tarball data for this package. The data will
    /// be checked for integrity based on the given [`Integrity`].  An
    /// [`std::io::Error`] with [`std::io::ErrorKind::InvalidData`] will be
    /// returned in case of integrity validation failure.
    pub async fn tarball_checked(&self, integrity: Integrity) -> Result<Tarball> {
        let data = self.fetcher.tarball(self).await?;
        Ok(Tarball::new(data, integrity))
    }

    /// A `Stream` of extracted entries from the `Package`'s tarball. The
    /// tarball stream will have its integrity validated based on package
    /// metadata. See [`Package::tarball`] for more information.
    pub async fn entries(&self) -> Result<Entries> {
        self.tarball().await?.entries()
    }

    /// A `Stream` of extracted entries from the `Package`'s tarball. The
    /// tarball stream will NOT have its integrity validated. See
    /// [`Package::tarball_unchecked`] for more information.
    pub async fn entries_unchecked(&self) -> Result<Entries> {
        self.tarball_unchecked().await?.entries()
    }

    /// A `Stream` of extracted entries from the `Package`'s tarball. The
    /// tarball stream will have its integrity validated based on
    /// [`Integrity`]. See [`Package::tarball_checked`] for more information.
    pub async fn entries_checked(&self, integrity: Integrity) -> Result<Entries> {
        self.tarball_checked(integrity).await?.entries()
    }

    /// Links an entire package directory from the cache into a destination.
    /// If the directory has not already been cached, the package will be
    /// downloaded and cached automatically.
    ///
    /// If the package resolution does not include integrity information, this
    /// will error.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn link_to_dir(&self, dir: &Path) -> Result<Integrity> {
        let Some(cache) = self.cache.as_deref() else {
            return Err(NassunError::NoCacheError);
        };
        let Some(sri) = self.resolved.integrity().cloned() else {
            return Err(NassunError::NoIntegrityError(Box::new(
                self.resolved.clone(),
            )));
        };
        let pkg_path = crate::tarball::linkable_tarball_dir(cache, &self.resolved, &sri);
        if self.link_from_cache(cache, dir, &sri).await.is_err() {
            // Try to download and cache again, then link.
            let dir_key = crate::tarball::tarball_dir_key(self.resolved(), &sri);
            self.tarball_checked(sri.clone())
                .await?
                .extract_from_tarball_data(&pkg_path, None, true)
                .await?
                .save(cache, &dir_key)?;

            self.link_from_cache(cache, dir, &sri).await?;
        }
        Ok(sri)
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn link_from_cache(&self, cache: &Path, dest: &Path, sri: &Integrity) -> Result<()> {
        std::fs::create_dir_all(dest.parent().expect("must have parent")).map_err(|e| {
            NassunError::ExtractIoError(
                e,
                Some(PathBuf::from(dest.parent().unwrap())),
                "creating destination directory for tarball.".into(),
            )
        })?;

        let source = crate::tarball::linkable_tarball_dir(cache, &self.resolved, sri);
        let source = source
            .canonicalize()
            .map_err(|e| NassunError::CanonicalizeError(source.clone(), e))?;
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&source, dest)
            .or_else(|_| junction::create(&source, dest))
            .map_err(|e| {
                NassunError::JunctionsNotSupported(source.to_owned(), dest.to_owned(), e)
            })?;
        #[cfg(unix)]
        std::os::unix::fs::symlink(&source, &dest).io_context(|| {
            format!(
                "Failed to create symlink while linking dependency, from {} to {}.",
                source.display(),
                dest.display()
            )
        })?;
        Ok(())
    }

    /// Extract tarball to a directory, optionally caching its contents.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn extract_to_dir(&self, dir: &Path, prefer_copy: bool) -> Result<Integrity> {
        if let Some(sri) = self.resolved.integrity() {
            if let Some(cache) = self.cache.as_deref() {
                if let Some(entry) = cacache::index::find(cache, &crate::tarball::tarball_key(sri))
                    .map_err(|e| NassunError::ExtractCacheError(e, None))?
                {
                    let sri = sri.clone();
                    // If extracting from the cache failed for some reason
                    // (bad data, etc), then go ahead and do a network
                    // extract.
                    match self
                        .extract_from_cache(dir, cache, entry, prefer_copy)
                        .await
                    {
                        Ok(_) => return Ok(sri),
                        Err(e) => {
                            tracing::warn!("extracting package {:?} from cache failed, possily due to cache corruption: {e}", self.resolved());
                            if let Some(entry) =
                                cacache::index::find(cache, &crate::tarball::tarball_key(&sri))
                                    .map_err(|e| NassunError::ExtractCacheError(e, None))?
                            {
                                tracing::debug!("removing corrupted cache entry.");
                                clean_from_cache(cache, &sri, entry)?;
                            }
                            return Ok(self
                                .tarball_checked(sri)
                                .await?
                                .extract_from_tarball_data(dir, self.cache.as_deref(), prefer_copy)
                                .await?
                                .integrity
                                .parse()?);
                        }
                    }
                } else {
                    return Ok(self
                        .tarball_checked(sri.clone())
                        .await?
                        .extract_from_tarball_data(dir, self.cache.as_deref(), prefer_copy)
                        .await?
                        .integrity
                        .parse()?);
                }
            }
            Ok(self
                .tarball_checked(sri.clone())
                .await?
                .extract_from_tarball_data(dir, self.cache.as_deref(), prefer_copy)
                .await?
                .integrity
                .parse()?)
        } else {
            Ok(self
                .tarball_unchecked()
                .await?
                .extract_from_tarball_data(dir, self.cache.as_deref(), prefer_copy)
                .await?
                .integrity
                .parse()?)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn extract_from_cache(
        &self,
        dir: &Path,
        cache: &Path,
        entry: cacache::Metadata,
        mut prefer_copy: bool,
    ) -> Result<()> {
        let dir = PathBuf::from(dir);
        let cache = PathBuf::from(cache);
        let name = self.name().to_owned();
        async_std::task::spawn_blocking(move || {
            let mut created = std::collections::HashSet::new();
            let index = rkyv::check_archived_root::<TarballIndex>(
                entry
                    .raw_metadata
                    .as_ref()
                    .ok_or_else(|| NassunError::CacheMissingIndexError(name))?,
            )
            .map_err(|e| NassunError::DeserializeCacheError(e.to_string()))?;
            prefer_copy = index.should_copy || prefer_copy;
            for (path, (sri, mode)) in index.files.iter() {
                let sri: Integrity = sri.parse()?;
                let path = dir.join(&path[..]);
                let parent = PathBuf::from(path.parent().expect("this will always have a parent"));
                if !created.contains(&parent) {
                    std::fs::create_dir_all(path.parent().expect("this will always have a parent"))
                        .map_err(|e| {
                            NassunError::ExtractIoError(
                                e,
                                Some(PathBuf::from(path.parent().unwrap())),
                                "creating destination directory for tarball.".into(),
                            )
                        })?;
                    created.insert(parent);
                }

                crate::tarball::extract_from_cache(&cache, &sri, &path, prefer_copy, *mode)?;
            }
            #[cfg(unix)]
            for binpath in index.bin_paths.iter() {
                {
                    crate::tarball::set_bin_mode(&dir.join(&binpath[..]))?;
                }
            }
            Ok::<_, NassunError>(())
        })
        .await?;
        Ok(())
    }
}

impl fmt::Debug for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Package")
            .field("from", &self.from)
            .field("name", &self.name)
            .field("resolved", &self.resolved)
            .field("base_dir", &self.resolved)
            .finish()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn clean_from_cache(cache: &Path, sri: &Integrity, entry: cacache::Metadata) -> Result<()> {
    let index = rkyv::check_archived_root::<TarballIndex>(
        entry
            .raw_metadata
            .as_ref()
            .ok_or_else(|| NassunError::CacheMissingIndexError("".into()))?,
    )
    .map_err(|e| NassunError::DeserializeCacheError(e.to_string()))?;
    for (sri, _) in index.files.values() {
        let sri: Integrity = sri.as_str().parse()?;
        match cacache::remove_hash_sync(cache, &sri) {
            Ok(_) => {}
            // We don't care if the file doesn't exist.
            Err(cacache::Error::IoError(e, _)) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(NassunError::ExtractCacheError(e, None));
            }
        }
    }
    cacache::remove_sync(cache, crate::tarball::tarball_key(sri))
        .map_err(|e| NassunError::ExtractCacheError(e, None))?;
    Ok(())
}
