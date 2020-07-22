use async_trait::async_trait;
use futures::io::AsyncRead;

use super::PackageFetcher;

use crate::error::Result;
use crate::package::{Manifest, Packument, Package, PackageRequest};

pub struct DirFetcher {}
impl DirFetcher {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PackageFetcher for DirFetcher {
    async fn manifest(&mut self, _pkg: &Package) -> Result<Manifest> {
        unimplemented!()
    }
    async fn packument(&mut self, _pkg: &PackageRequest) -> Result<Packument> {
        unimplemented!()
    }
    async fn tarball(&mut self, _pkg: &Package) -> Result<Box<dyn AsyncRead + Send + Sync>> {
        unimplemented!()
    }
}
