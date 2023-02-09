use std::fmt;
use std::path::PathBuf;

use async_std::sync::Arc;
use oro_common::{CorgiPackument, CorgiVersionMetadata, Packument, VersionMetadata};
use oro_package_spec::PackageSpec;
use ssri::Integrity;

use crate::entries::Entries;
use crate::error::Result;
use crate::fetch::PackageFetcher;
use crate::resolver::PackageResolution;
use crate::tarball::Tarball;

/// A resolved package. A concrete version has been determined from its
/// PackageSpec by the version resolver.
pub struct Package {
    pub(crate) from: PackageSpec,
    pub(crate) name: String,
    pub(crate) resolved: PackageResolution,
    pub(crate) fetcher: Arc<dyn PackageFetcher>,
    pub(crate) base_dir: PathBuf,
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

    /// [`AsyncRead`] of the raw tarball data for this package. The data will
    /// not be checked for integrity based on the current `Package`'s
    /// [`Integrity`]. That is, bad or incomplete data may be returned.
    pub async fn tarball_unchecked(&self) -> Result<Tarball> {
        let data = self.fetcher.tarball(self).await?;
        Ok(Tarball::new_unchecked(data))
    }

    /// [`AsyncRead`] of the raw tarball data for this package. The data will
    /// be checked for integrity based on the current `Package`'s
    /// [`Integrity`], if present in its [`Package::metadata`]. An
    /// [`std::io::Error`] with [`std::io::ErrorKind::InvalidData`] will be
    /// returned in case of integrity validation failure.
    pub async fn tarball(&self) -> Result<Tarball> {
        let data = self.fetcher.tarball(self).await?;
        if let Some(integrity) = self.corgi_metadata().await?.dist.integrity {
            if let Ok(integrity) = integrity.parse::<Integrity>() {
                Ok(Tarball::new(data, integrity))
            } else {
                self.tarball_unchecked().await
            }
        } else {
            self.tarball_unchecked().await
        }
    }

    /// [`AsyncRead`] of the raw tarball data for this package. The data will
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
}

impl fmt::Debug for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Package")
            .field("from", &self.from)
            .field("name", &self.name)
            .field("resolved", &self.resolved)
            .finish()
    }
}
