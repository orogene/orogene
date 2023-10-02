use std::fmt;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::path::PathBuf;

use async_std::sync::Arc;
use oro_common::{CorgiPackument, CorgiVersionMetadata, Packument, VersionMetadata};
use oro_package_spec::PackageSpec;
use ssri::Integrity;

use crate::entries::Entries;
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

    /// Extract tarball to a directory, optionally caching its contents. The
    /// tarball stream will have its integrity validated based on package
    /// metadata. See [`Package::tarball`] for more information.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn extract_to_dir(
        &self,
        dir: impl AsRef<Path>,
        prefer_copy: bool,
    ) -> Result<Integrity> {
        async fn inner(me: &Package, dir: &Path, prefer_copy: bool) -> Result<Integrity> {
            me.extract_to_dir_inner(dir, me.resolved.integrity(), prefer_copy)
                .await
        }
        inner(self, dir.as_ref(), prefer_copy).await
    }

    /// Extract tarball to a directory, optionally caching its contents. The
    /// tarball stream will NOT have its integrity validated. See
    /// [`Package::tarball_unchecked`] for more information.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn extract_to_dir_unchecked(
        &self,
        dir: impl AsRef<Path>,
        prefer_copy: bool,
    ) -> Result<Integrity> {
        async fn inner(me: &Package, dir: &Path, prefer_copy: bool) -> Result<Integrity> {
            me.extract_to_dir_inner(dir, None, prefer_copy).await
        }
        inner(self, dir.as_ref(), prefer_copy).await
    }

    /// Extract tarball to a directory, optionally caching its contents. The
    /// tarball stream will have its integrity validated based on
    /// [`Integrity`]. See [`Package::tarball_checked`] for more information.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn extract_to_dir_checked(
        &self,
        dir: impl AsRef<Path>,
        sri: Integrity,
        prefer_copy: bool,
    ) -> Result<Integrity> {
        async fn inner(
            me: &Package,
            dir: &Path,
            sri: Integrity,
            prefer_copy: bool,
        ) -> Result<Integrity> {
            me.extract_to_dir_inner(dir, Some(&sri), prefer_copy).await
        }
        inner(self, dir.as_ref(), sri, prefer_copy).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn extract_to_dir_inner(
        &self,
        dir: &Path,
        integrity: Option<&Integrity>,
        prefer_copy: bool,
    ) -> Result<Integrity> {
        if let Some(sri) = integrity {
            if let Some(cache) = self.cache.as_deref() {
                if let Some(entry) = cacache::index::find(cache, &crate::tarball::tarball_key(sri))
                    .map_err(|e| NassunError::ExtractCacheError(e, None))?
                {
                    let sri = sri.clone();
                    match self
                        .extract_from_cache(dir, cache, entry, prefer_copy)
                        .await
                    {
                        Ok(_) => return Ok(sri),
                        // If extracting from the cache failed for some reason
                        // (bad data, etc), then go ahead and do a network
                        // extract.
                        Err(e) => {
                            tracing::warn!("extracting package {:?} from cache failed, possily due to cache corruption: {e}", self.resolved());
                            if let Some(entry) =
                                cacache::index::find(cache, &crate::tarball::tarball_key(&sri))
                                    .map_err(|e| NassunError::ExtractCacheError(e, None))?
                            {
                                tracing::debug!("removing corrupted cache entry.");
                                clean_from_cache(cache, &sri, entry)?;
                            }
                            return self
                                .tarball_checked(sri)
                                .await?
                                .extract_from_tarball_data(dir, self.cache.as_deref(), prefer_copy)
                                .await;
                        }
                    }
                } else {
                    return self
                        .tarball_checked(sri.clone())
                        .await?
                        .extract_from_tarball_data(dir, self.cache.as_deref(), prefer_copy)
                        .await;
                }
            }
            self.tarball_checked(sri.clone())
                .await?
                .extract_from_tarball_data(dir, self.cache.as_deref(), prefer_copy)
                .await
        } else {
            self.tarball_unchecked()
                .await?
                .extract_from_tarball_data(dir, self.cache.as_deref(), prefer_copy)
                .await
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
            for (archived_path, (sri, mode)) in index.files.iter() {
                let sri: Integrity = sri.parse()?;
                let path = dir.join(&archived_path[..]);
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

                let mode = if index.bin_paths.contains(archived_path) {
                    *mode | 0o111
                } else {
                    *mode
                };

                crate::tarball::extract_from_cache(&cache, &sri, &path, prefer_copy, mode)?;
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
