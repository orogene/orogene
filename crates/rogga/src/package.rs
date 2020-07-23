use std::path::PathBuf;

use async_std::sync::RwLock;
use futures::io::AsyncRead;
use http_types::Url;
use package_arg::PackageArg;
use semver::Version;
use ssri::Integrity;

use crate::error::Result;
use crate::fetch::PackageFetcher;
use crate::packument::Packument;

#[derive(Clone, Debug)]
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

    // idk what `resolved` should be here? Probably an actual PackageVersion?
    pub async fn resolve(self, resolved: PackageResolution) -> Result<Package> {
        let name = self.name().await?;
        // Is this really necessary? Maybe not all types need this??
        let packument = self.packument().await?;
        Ok(Package {
            from: self.spec,
            name,
            resolved,
            fetcher: self.fetcher,
            packument,
            manifest: RwLock::new(None),
        })
    }
}

/// Represents a fully-resolved, specific version of a package as it would be fetched.
#[derive(Clone, Debug)]
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
    pub packument: Packument,
    manifest: RwLock<Option<Manifest>>,
    pub(crate) fetcher: RwLock<Box<dyn PackageFetcher>>,
}
impl Package {
    pub async fn manifest(&self) -> Result<Manifest> {
        let read_lock = self.manifest.read().await;
        if let Some(manifest) = read_lock.clone() {
            Ok(manifest)
        } else {
            std::mem::drop(read_lock);
            let mut manifest = self.manifest.write().await;
            *manifest = Some(self.fetcher.write().await.manifest(&self).await?);
            Ok(manifest.clone().unwrap())
        }
    }

    pub async fn tarball(&self) -> Result<Box<dyn AsyncRead + Send + Sync>> {
        self.fetcher.write().await.tarball(&self).await
    }
}
