use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

use oro_package_spec::PackageSpec;

use crate::error::SessError;
use crate::fetch::PackageFetcher;
use crate::package::Package;
use crate::packument::Packument;
use crate::resolver::{PackageResolution, PackageResolver};

/// A package request from which more information can be derived. PackageRequest objects can be resolved into a `Package` by using a `PackageResolver`
pub struct PackageRequest {
    pub(crate) name: String,
    pub(crate) spec: PackageSpec,
    pub(crate) base_dir: PathBuf,
    pub(crate) fetcher: Arc<dyn PackageFetcher>,
}

impl PackageRequest {
    pub fn spec(&self) -> &PackageSpec {
        &self.spec
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn base_dir(&self) -> &PathBuf {
        &self.base_dir
    }

    /// Returns the packument with general metadata about the package and its
    /// various versions.
    pub async fn packument(&self) -> Result<Arc<Packument>, SessError> {
        self.fetcher.packument(&self.spec, &self.base_dir).await
    }

    pub async fn resolve_with<T: PackageResolver>(self, resolver: &T) -> Result<Package, SessError> {
        let resolution = resolver.resolve(&self).await?;
        self.resolve_to(resolution)
    }

    pub fn resolve_to(self, resolved: PackageResolution) -> Result<Package, SessError> {
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
