use std::fmt;

use futures::io::AsyncRead;
use oro_package_spec::PackageSpec;

use crate::error::Result;
use crate::fetch::PackageFetcherPool;
use crate::packument::VersionMetadata;
use crate::resolver::PackageResolution;

/// A resolved package. A concrete version has been determined from its
/// PackageSpec by the version resolver.
pub struct Package {
    pub from: PackageSpec,
    pub name: String,
    pub resolved: PackageResolution,
    pub(crate) fetcher_pool: PackageFetcherPool,
}

impl Package {
    pub async fn metadata(&self) -> Result<VersionMetadata> {
        self.fetcher_pool.get().await.metadata(&self).await
    }

    pub async fn tarball(&self) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>> {
        self.fetcher_pool.get().await.tarball(&self).await
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
