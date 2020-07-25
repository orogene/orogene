use async_trait::async_trait;
use futures::io::AsyncRead;

use crate::error::Result;
use crate::package::{Package, PackageRequest};
use crate::packument::{Manifest, Packument};

pub use dir::DirFetcher;
pub use registry::RegistryFetcher;

mod dir;
mod registry;

#[async_trait]
pub trait PackageFetcher: Send + Sync {
    async fn manifest(&mut self, pkg: &Package) -> Result<Manifest>;
    async fn packument(&mut self, pkg: &PackageRequest) -> Result<Packument>;
    async fn tarball(&mut self, pkg: &Package) -> Result<Box<dyn AsyncRead + Send + Sync>>;
}
