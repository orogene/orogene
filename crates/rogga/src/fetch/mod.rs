use async_trait::async_trait;
use futures::io::AsyncRead;
use package_spec::PackageSpec;

use crate::error::Result;
use crate::package::Package;
use crate::packument::{Packument, VersionMetadata};

pub use dir::DirFetcher;
pub use registry::RegistryFetcher;

mod dir;
mod registry;

#[async_trait]
pub trait PackageFetcher: std::fmt::Debug + Send + Sync {
    async fn name(&mut self, spec: &PackageSpec) -> Result<String>;
    async fn metadata(&mut self, pkg: &Package) -> Result<VersionMetadata>;
    async fn packument(&mut self, pkg: &PackageSpec) -> Result<Packument>;
    async fn tarball(&mut self, pkg: &Package) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>>;
}
