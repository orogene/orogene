use async_trait::async_trait;
use futures::io::AsyncRead;
use package_arg::PackageArg;

use crate::error::Result;
use crate::package::{Package, PackageRequest};
use crate::packument::{Packument, VersionMetadata};

pub use dir::DirFetcher;
pub use registry::RegistryFetcher;

mod dir;
mod registry;

#[async_trait]
pub trait PackageFetcher: Send + Sync {
    async fn name(&mut self, spec: &PackageArg) -> Result<String>;
    async fn manifest(&mut self, pkg: &Package) -> Result<VersionMetadata>;
    async fn packument(&mut self, pkg: &PackageRequest) -> Result<Packument>;
    async fn tarball(&mut self, pkg: &Package) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>>;
}
