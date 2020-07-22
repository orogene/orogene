use std::path::PathBuf;

use async_std::sync::RwLock;
use futures::io::AsyncRead;
use http_types::Url;
use package_arg::PackageArg;
use semver::Version;
use serde::Deserialize;
use ssri::Integrity;

use crate::error::Result;
use crate::fetch::PackageFetcher;

#[derive(Clone, Debug, Deserialize)]
pub struct Manifest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub integrity: Option<Integrity>,
    pub resolved: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Packument {
    pub name: Option<String>,
}

// Should this be an enum that mostly copies PackageArg? Should this replace
// PackageArg itself? Should this just expose PackageArg through a .get()
// method that returns a reference?
pub struct PackageRequest {
    pub(crate) name: RwLock<Option<String>>,
    pub(crate) packument: RwLock<Option<Packument>>,
    pub(crate) spec: PackageArg,
    pub(crate) fetcher: RwLock<Box<dyn PackageFetcher>>,
}

impl PackageRequest {
    pub fn get_spec(&self) -> &PackageArg {
        &self.spec
    }

    pub async fn name(&self) -> Result<String> {
        if let Some(name) = self.name.read().await.clone() {
            Ok(name)
        } else {
            use PackageArg::*;
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
        if let Some(packument) = self.packument.read().await.clone() {
            Ok(packument)
        } else {
            let mut packument = self.packument.write().await;
            *packument = Some(self.fetcher.write().await.packument(&self).await?);
            Ok(packument.clone().unwrap())
        }
    }

    // idk what `resolved` should be here? Probably an actual PackageVersion?
    pub fn resolve(self, _resolved: String) -> Package {
        todo!()
    }
}

pub enum PackageVersion {
    Npm { version: Version, tarball: Url },
    Dir { path: PathBuf },
}

/// A resolved package. A concrete version has been determined from its
/// PackageArg by the version resolver.
pub struct Package {
    pub from: PackageArg,
    pub name: String,
    pub version: PackageVersion,
    pub(crate) fetcher: RwLock<Box<dyn PackageFetcher>>,
}
impl Package {
    pub async fn manifest(&self) -> Result<Manifest> {
        todo!()
    }
    pub async fn tarball(&self) -> Result<Box<dyn AsyncRead + Send + Sync>> {
        self.fetcher.write().await.tarball(&self).await
    }
}
