use std::path::PathBuf;

use async_std::sync::RwLock;
use async_trait::async_trait;
use futures::io::AsyncRead;
use package_arg::PackageArg;
use semver::Version;
use serde::{Serialize, Deserialize};
use ssri::Integrity;
use thiserror::Error;

use crate::error::Result;
use crate::fetch::PackageFetcher;
use crate::packument::Packument;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub name: Option<String>,
    pub version: Option<Version>,
    pub integrity: Option<Integrity>,
    pub resolved: PackageResolution,
}

// Should this be an enum that mostly copies PackageArg? Should this replace
// PackageArg itself? Should this just expose PackageArg through a .get()
// method that returns a reference?
pub struct PackageRequest {
    pub(crate) name: RwLock<Option<String>>,
    pub(crate) spec: PackageArg,
    pub(crate) fetcher: RwLock<Box<dyn PackageFetcher>>,
}

impl PackageRequest {
    pub fn spec(&self) -> &PackageArg {
        &self.spec
    }

    pub async fn name(&self) -> Result<String> {
        let read_name = self.name.read().await;
        if let Some(name) = read_name.clone() {
            Ok(name)
        } else {
            use PackageArg::*;
            std::mem::drop(read_name);
            let mut name = self.name.write().await;
            *name = Some(match self.spec {
                Dir { ref path } => self.packument().await?.name.unwrap_or_else(|| {
                    if let Some(name) = path.file_name() {
                        name.to_string_lossy().into()
                    } else {
                        "".into()
                    }
                }),
                Alias { ref name, .. } | Npm { ref name, .. } => name.clone(),
            });
            Ok(name.clone().unwrap())
        }
    }

    /// Returns the packument with general metadata about the package and its
    /// various versions.
    pub async fn packument(&self) -> Result<Packument> {
        self.fetcher.write().await.packument(&self).await
    }

    pub async fn resolve_with<T: Resolver>(self, resolver: T) -> Result<Package> {
        let resolution = resolver.resolve(&self).await?;
        self.resolve_to(resolution).await
    }

    pub async fn resolve_to(self, resolved: PackageResolution) -> Result<Package> {
        let name = self.name().await?;
        Ok(Package {
            from: self.spec,
            name,
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
pub trait Resolver {
    async fn resolve(
        &self,
        wanted: &PackageRequest,
    ) -> std::result::Result<PackageResolution, ResolverError>;
}

/// Represents a fully-resolved, specific version of a package as it would be fetched.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PackageResolution {
    Npm { version: Version, tarball: String },
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

    pub async fn tarball(&self) -> Result<Box<dyn AsyncRead + Send + Sync>> {
        self.fetcher.write().await.tarball(&self).await
    }
}
