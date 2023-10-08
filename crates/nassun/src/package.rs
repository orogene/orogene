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

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtractMode {
    /// Automatically decide whether to Copy or Reflink, based on fallbacks. Will never hardlink.
    #[default]
    Auto,
    /// Copy contents from the cache in their entirety.
    Copy,
    /// Reflink contents from the cache instead of doing full copies.
    Reflink,
    /// Try to hard link contents from the cache. Fall back to reflink, then copy if that fails.
    AutoHardlink,
    /// Hard link contents from the cache instead of doing full copies.
    Hardlink,
}

#[cfg(not(target_arch = "wasm32"))]
impl ExtractMode {
    pub fn is_copy(&self) -> bool {
        matches!(
            self,
            ExtractMode::Copy | ExtractMode::Auto | ExtractMode::Reflink
        )
    }
}

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
        extract_mode: ExtractMode,
    ) -> Result<Integrity> {
        async fn inner(me: &Package, dir: &Path, extract_mode: ExtractMode) -> Result<Integrity> {
            me.extract_to_dir_inner(dir, me.resolved.integrity(), extract_mode)
                .await
        }
        inner(self, dir.as_ref(), extract_mode).await
    }

    /// Extract tarball to a directory, optionally caching its contents. The
    /// tarball stream will NOT have its integrity validated. See
    /// [`Package::tarball_unchecked`] for more information.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn extract_to_dir_unchecked(
        &self,
        dir: impl AsRef<Path>,
        extract_mode: ExtractMode,
    ) -> Result<Integrity> {
        async fn inner(me: &Package, dir: &Path, extract_mode: ExtractMode) -> Result<Integrity> {
            me.extract_to_dir_inner(dir, None, extract_mode).await
        }
        inner(self, dir.as_ref(), extract_mode).await
    }

    /// Extract tarball to a directory, optionally caching its contents. The
    /// tarball stream will have its integrity validated based on
    /// [`Integrity`]. See [`Package::tarball_checked`] for more information.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn extract_to_dir_checked(
        &self,
        dir: impl AsRef<Path>,
        sri: Integrity,
        extract_mode: ExtractMode,
    ) -> Result<Integrity> {
        async fn inner(
            me: &Package,
            dir: &Path,
            sri: Integrity,
            extract_mode: ExtractMode,
        ) -> Result<Integrity> {
            me.extract_to_dir_inner(dir, Some(&sri), extract_mode).await
        }
        inner(self, dir.as_ref(), sri, extract_mode).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn extract_to_dir_inner(
        &self,
        dir: &Path,
        integrity: Option<&Integrity>,
        extract_mode: ExtractMode,
    ) -> Result<Integrity> {
        if let Some(sri) = integrity {
            if let Some(cache) = self.cache.as_deref() {
                if let Some(entry) = cacache::index::find(cache, &crate::tarball::tarball_key(sri))
                    .map_err(|e| NassunError::ExtractCacheError(e, None))?
                {
                    let sri = sri.clone();
                    match self
                        .extract_from_cache(dir, cache, entry, extract_mode)
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
                                .extract_from_tarball_data(dir, self.cache.as_deref(), extract_mode)
                                .await;
                        }
                    }
                } else {
                    return self
                        .tarball_checked(sri.clone())
                        .await?
                        .extract_from_tarball_data(dir, self.cache.as_deref(), extract_mode)
                        .await;
                }
            }
            self.tarball_checked(sri.clone())
                .await?
                .extract_from_tarball_data(dir, self.cache.as_deref(), extract_mode)
                .await
        } else {
            self.tarball_unchecked()
                .await?
                .extract_from_tarball_data(dir, self.cache.as_deref(), extract_mode)
                .await
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn extract_from_cache(
        &self,
        dir: &Path,
        cache: &Path,
        entry: cacache::Metadata,
        mut extract_mode: ExtractMode,
    ) -> Result<()> {
        let dir = PathBuf::from(dir);
        let cache = PathBuf::from(cache);
        let name = self.name().to_owned();
        async_std::task::spawn_blocking(move || {
            let created = dashmap::DashSet::new();
            let index = rkyv::check_archived_root::<TarballIndex>(
                entry
                    .raw_metadata
                    .as_ref()
                    .ok_or_else(|| NassunError::CacheMissingIndexError(name))?,
            )
            .map_err(|e| NassunError::DeserializeCacheError(e.to_string()))?;
            extract_mode = if index.should_copy && !extract_mode.is_copy() {
                // In general, if reflinks are supported, we would have
                // received them as extract_mode already. So there's no need
                // to try and do a fallback here.
                ExtractMode::Copy
            } else {
                extract_mode
            };
            for (archived_path, (sri, mode)) in index.files.iter() {
                let sri: Integrity = sri.parse()?;
                let path = dir.join(&archived_path[..]);
                let parent = PathBuf::from(path.parent().expect("this will always have a parent"));
                crate::tarball::mkdirp(&parent, &created)?;

                let mode = if index.bin_paths.contains(archived_path) {
                    *mode | 0o111
                } else {
                    *mode
                };

                crate::tarball::extract_from_cache(&cache, &sri, &path, extract_mode, mode)?;
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
