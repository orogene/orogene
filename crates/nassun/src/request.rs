use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use async_std::sync::Arc;
use oro_common::Packument;
use oro_package_spec::PackageSpec;

use crate::error::Result;
use crate::fetch::PackageFetcher;
use crate::package::Package;
use crate::resolver::{PackageResolution, PackageResolver};

/// A package request from which more information can be derived.
/// `PackageRequest` objects can be resolved into a [`Package`] by using a
/// [`PackageResolver`], or directly by using a [`PackageResolution`].
pub struct PackageRequest {
    pub(crate) name: String,
    pub(crate) spec: PackageSpec,
    pub(crate) base_dir: PathBuf,
    pub(crate) fetcher: Arc<dyn PackageFetcher>,
}

impl PackageRequest {
    /// [`PackageSpec`] that this request was created for.
    pub fn spec(&self) -> &PackageSpec {
        &self.spec
    }

    /// The name of the package, as it should be used in the dependency graph.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The base directory that this request will use for resolving relative
    /// paths for directory dependencies.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Returns the packument with general metadata about the package and its
    /// various versions.
    pub async fn packument(&self) -> Result<Arc<Packument>> {
        self.fetcher.packument(&self.spec, &self.base_dir).await
    }

    /// Resolve to a [`Package`] using the given [`PackageResolver`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> miette::Result<()> {
    /// use oro_classic_resolver::ClassicResolver;
    /// use nassun::Nassun;
    ///
    /// let pkg = Nassun::new()
    ///     .request("debug@^4.1.1")
    ///     .await?
    ///     .resolve_with(&ClassicResolver::new())
    ///     .await?
    ///     .metadata()
    ///     .await?
    ///     .manifest;
    ///
    /// assert_eq!(pkg.name, Some("debug".into()));
    /// assert_eq!(pkg.version, Some("4.1.1".parse()?));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn resolve_with<T: PackageResolver>(self, resolver: &T) -> Result<Package> {
        let resolution = resolver.resolve(&self).await?;
        self.resolve_to(resolution)
    }

    /// Resolve to a [`Package`] using the given an already-calculated
    /// [`PackageResolution`].
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
