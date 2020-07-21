use async_trait::async_trait;
use futures::io::AsyncRead;

use super::PackageFetcher;

use crate::data::{Manifest, Packument};
use crate::error::Result;
use crate::package::Package;

pub struct DirFetcher {}
impl DirFetcher {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PackageFetcher for DirFetcher {
    async fn manifest(&mut self, pkg: &Package) -> Result<Manifest> {
        unimplemented!()
    }
    async fn packument(&mut self, pkg: &Package) -> Result<Packument> {
        unimplemented!()
    }
    async fn tarball(&mut self, pkg: &Package) -> Result<Box<dyn AsyncRead + Send + Sync>> {
        unimplemented!()
    }
}
