use async_trait::async_trait;
use futures::io::AsyncRead;

use super::{Manifest, PackageFetcher, PackageFetcherError, Packument};

pub struct DirFetcher {}
impl DirFetcher {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PackageFetcher for DirFetcher {
    async fn manifest(&self) -> Result<Manifest, PackageFetcherError> {
        unimplemented!()
    }
    async fn packument(&self) -> Result<Packument, PackageFetcherError> {
        unimplemented!()
    }
    async fn tarball(&self) -> Result<Box<dyn AsyncRead + Send + Sync>, PackageFetcherError> {
        unimplemented!()
    }
}
