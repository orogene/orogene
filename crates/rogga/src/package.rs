use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use async_std::sync::RwLock;
use async_trait::async_trait;
use futures::io::AsyncRead;
use http_types::Url;
use oro_node_semver::Version;
use package_spec::PackageSpec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::error::Result;
use crate::fetch::PackageFetcher;
use crate::packument::{Packument, VersionMetadata};

/// A package request from which more information can be derived. PackageRequest objects can be resolved into a `Package` by using a `PackageResolver`
#[derive(Debug)]
pub struct PackageRequest {
    pub(crate) name: String,
    pub(crate) spec: PackageSpec,
    pub(crate) fetcher: RwLock<Box<dyn PackageFetcher>>,
}

impl PackageRequest {
    pub fn spec(&self) -> &PackageSpec {
        &self.spec
    }

    // TODO: do this before instantiating the PackageRequest
    pub fn name(&self) -> &String {
        &self.name
    }

    /// Returns the packument with general metadata about the package and its
    /// various versions.
    pub async fn packument(&self) -> Result<Packument> {
        self.fetcher.write().await.packument(&self.spec).await
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

impl PartialEq for PackageRequest {
    fn eq(&self, other: &PackageRequest) -> bool {
        self.name() == other.name() && self.spec().target() == other.spec().target()
    }
}

impl Eq for PackageRequest {}

impl Hash for PackageRequest {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.spec().target().hash(state);
    }
}

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("No matching version found for spec {name}@{spec:?} in {versions:#?}.")]
    NoVersion {
        name: String,
        spec: PackageSpec,
        versions: Vec<String>,
    },
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

#[async_trait]
impl<F> PackageResolver for F
where
    F: Fn(&PackageRequest) -> std::result::Result<PackageResolution, ResolverError> + Sync + Send,
{
    async fn resolve(
        &self,
        wanted: &PackageRequest,
    ) -> std::result::Result<PackageResolution, ResolverError> {
        self(wanted)
    }
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
    pub from: PackageSpec,
    pub name: String,
    pub resolved: PackageResolution,
    pub(crate) fetcher: RwLock<Box<dyn PackageFetcher>>,
}
impl Package {
    pub async fn metadata(&self) -> Result<VersionMetadata> {
        self.fetcher.write().await.metadata(&self).await
    }

    pub async fn tarball(&self) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>> {
        self.fetcher.write().await.tarball(&self).await
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
