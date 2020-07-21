use async_std::sync::RwLock;
use futures::io::AsyncRead;
use package_arg::PackageArg;

use crate::data::{Manifest, Packument};
use crate::error::Result;
use crate::fetch::PackageFetcher;

pub struct Package {
    pub(crate) spec: PackageArg,
    pub(crate) fetcher: RwLock<Box<dyn PackageFetcher>>,
}

/// A representation of a particular package, as requested.
impl Package {
    /// Fetches the canonical name for this Package. That is, the name node
    /// would use to load it. This method is fallible because it might need to
    /// make filesystem or network queries in order to resolve to an actual
    /// name.
    pub async fn name(&self) -> Result<String> {
        use PackageArg::*;
        match self.spec {
            Dir { ref path } => Ok(self.manifest().await?.name.unwrap_or_else(|| {
                if let Some(name) = path.file_name() {
                    name.to_string_lossy().into()
                } else {
                    "".into()
                }
            })),
            Alias { ref name, .. } | Npm { ref name, .. } => Ok(name.clone()),
        }
    }

    /// Resolves this package from a request into a concrete name, version,
    /// and fetch location.
    pub async fn resolve(&mut self) -> Result<()> {
        todo!()
    }

    /// Returns a standardized Manifest with general information about the
    /// package.
    pub async fn manifest(&self) -> Result<Manifest> {
        self.fetcher.write().await.manifest(&self).await
    }

    /// Returns the packument with general metadata about the package and its
    /// various versions.
    pub async fn packument(&self) -> Result<Packument> {
        self.fetcher.write().await.packument(&self).await
    }

    /// Returns an AsyncRead of package data.
    pub async fn tarball(&self) -> Result<Box<dyn AsyncRead + Send + Sync>> {
        self.fetcher.write().await.tarball(&self).await
    }
}
