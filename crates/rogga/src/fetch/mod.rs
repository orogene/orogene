use async_trait::async_trait;
use futures::io::AsyncRead;
use ssri::Integrity;
use thiserror::Error;

pub use dir::DirFetcher;
pub use registry::RegistryFetcher;

mod dir;
mod registry;

pub struct Manifest {
    pub name: String,
    pub version: String,
    pub integrity: Option<Integrity>,
    pub resolved: String,
}
pub struct Packument {}

#[derive(Debug, Error)]
pub enum PackageFetcherError {}

#[async_trait]
pub trait PackageFetcher {
    async fn manifest(&self) -> Result<Manifest, PackageFetcherError>;
    async fn packument(&self) -> Result<Packument, PackageFetcherError>;
    async fn tarball(&self) -> Result<Box<dyn AsyncRead + Send + Sync>, PackageFetcherError>;
}
