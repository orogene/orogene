use std::path::Path;

use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use futures::AsyncRead;
use oro_client::{self, OroClient};
use oro_package_spec::PackageSpec;

use crate::{error::Result, fetch::PackageFetcher, Package, Packument, VersionMetadata};

#[derive(Debug)]
pub struct GitFetcher {
    client: Arc<Mutex<OroClient>>,
}

impl GitFetcher {
    pub fn new(client: Arc<Mutex<OroClient>>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl PackageFetcher for GitFetcher {
    async fn name(&self, _spec: &PackageSpec, _base_dir: &Path) -> Result<String> {
        todo!()
    }

    async fn metadata(&self, _pkg: &Package) -> Result<VersionMetadata> {
        todo!()
    }

    async fn packument(&self, _pkg: &PackageSpec, _base_dir: &Path) -> Result<Arc<Packument>> {
        todo!()
    }

    async fn tarball(
        &self,
        _pkg: &crate::Package,
    ) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>> {
        todo!()
    }
}
