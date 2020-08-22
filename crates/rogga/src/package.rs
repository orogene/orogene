use std::path::PathBuf;

use async_std::sync::RwLock;
use async_trait::async_trait;
use futures::io::AsyncRead;
use http_types::Url;
use oro_node_semver::Version;
use package_arg::PackageArg;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::error::Result;
use crate::fetch::PackageFetcher;
use crate::packument::{Manifest, Packument};

/// A package request from which more information can be derived. PackageRequest objects can be resolved into a `Package` by using a `PackageResolver`
pub struct PackageRequest {
    pub(crate) name: String,
    pub(crate) spec: PackageArg,
    pub(crate) fetcher: RwLock<Box<dyn PackageFetcher>>,
}

impl PackageRequest {
    pub fn spec(&self) -> &PackageArg {
        &self.spec
    }

    // TODO: do this before instantiating the PackageRequest
    pub fn name(&self) -> &String {
        &self.name
    }

    /// Returns the packument with general metadata about the package and its
    /// various versions.
    pub async fn packument(&self) -> Result<Packument> {
        self.fetcher.write().await.packument(&self).await
    }

    pub async fn resolve_with<T: PackageResolver>(self, resolver: &T) -> Result<Package> {
        let resolution = resolver.resolve(&self).await?;
        self.resolve_to(resolution)
    }

    pub fn resolve_to(self, resolved: PackageResolution) -> Result<Package> {
        Ok(Package {
            from: self.spec,
            name: self.name,
            resolved,
            fetcher: self.fetcher,
        })
    }
}

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("No matching version.")]
    NoVersion,
    #[error(transparent)]
    OtherError(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[async_trait]
pub trait PackageResolver {
    async fn resolve(
        &self,
        wanted: &PackageRequest,
    ) -> std::result::Result<PackageResolution, ResolverError>;
}

/// Represents a fully-resolved, specific version of a package as it would be fetched.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PackageResolution {
    Npm { version: Version, tarball: Url },
    Dir { path: PathBuf },
}

/// A resolved package. A concrete version has been determined from its
/// PackageArg by the version resolver.
pub struct Package {
    pub from: PackageArg,
    pub name: String,
    pub resolved: PackageResolution,
    pub(crate) fetcher: RwLock<Box<dyn PackageFetcher>>,
}
impl Package {
    pub async fn manifest(&self) -> Result<Manifest> {
        self.fetcher.write().await.manifest(&self).await
    }

    pub async fn tarball(&self) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>> {
        self.fetcher.write().await.tarball(&self).await
    }
}
