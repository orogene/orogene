use std::fmt;
use std::sync::Arc;

use oro_common::futures::io::AsyncRead;
use oro_package_spec::PackageSpec;

use crate::error::TorusError;
use crate::fetch::PackageFetcher;
use crate::packument::VersionMetadata;
use crate::resolver::PackageResolution;

/// A resolved package. A concrete version has been determined from its
/// PackageSpec by the version resolver.
pub struct Package {
    pub(crate) from: PackageSpec,
    pub(crate) name: String,
    pub(crate) resolved: PackageResolution,
    pub(crate) fetcher: Arc<dyn PackageFetcher>,
}

impl Package {
    pub fn from(&self) -> &PackageSpec {
        &self.from
    }

    pub fn name(&self) -> &str {
        &self.name[..]
    }

    pub fn resolved(&self) -> &PackageResolution {
        &self.resolved
    }

    pub async fn metadata(&self) -> Result<VersionMetadata, TorusError> {
        self.fetcher.metadata(self).await
    }

    pub async fn tarball(&self) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>, TorusError> {
        self.fetcher.tarball(self).await
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
