use std::path::Path;

use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use oro_client::{self, OroClient};
use oro_package_spec::PackageSpec;

use crate::fetch::PackageFetcher;

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
    async fn name(&self, _spec: &PackageSpec, _base_dir: &Path) -> crate::error::Result<String> {
        todo!()
    }

    async fn metadata(
        &self,
        _pkg: &crate::Package,
    ) -> crate::error::Result<crate::VersionMetadata> {
        todo!()
    }

    async fn packument(
        &self,
        _pkg: &PackageSpec,
        _base_dir: &Path,
    ) -> crate::error::Result<Arc<crate::Packument>> {
        todo!()
    }

    async fn tarball(
        &self,
        _pkg: &crate::Package,
    ) -> crate::error::Result<Box<dyn futures::AsyncRead + Unpin + Send + Sync>> {
        todo!()
    }
}
