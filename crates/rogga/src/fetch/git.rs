use std::path::Path;

use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use deadpool::managed::{Manager, RecycleResult};
use oro_client::{self, OroClient};
use oro_package_spec::PackageSpec;

use crate::error::Error;
use crate::fetch::PackageFetcher;

pub struct GitFetcherPool {
    client: Arc<Mutex<OroClient>>,
}

impl GitFetcherPool {
    pub fn new(client: Arc<Mutex<OroClient>>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Manager<Box<dyn PackageFetcher>, Error> for GitFetcherPool {
    async fn create(&self) -> Result<Box<dyn PackageFetcher>, Error> {
        Ok(Box::new(GitFetcher::new(self.client.clone())))
    }

    async fn recycle(&self, _fetcher: &mut Box<dyn PackageFetcher>) -> RecycleResult<Error> {
        Ok(())
    }
}

#[derive(Debug)]
struct GitFetcher {
    client: Arc<Mutex<OroClient>>,
}

impl GitFetcher {
    pub fn new(client: Arc<Mutex<OroClient>>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl PackageFetcher for GitFetcher {
    async fn name(
        &mut self,
        _spec: &PackageSpec,
        _base_dir: &Path,
    ) -> crate::error::Result<String> {
        todo!()
    }

    async fn metadata(
        &mut self,
        _pkg: &crate::Package,
    ) -> crate::error::Result<crate::VersionMetadata> {
        todo!()
    }

    async fn packument(
        &mut self,
        _pkg: &PackageSpec,
        _base_dir: &Path,
    ) -> crate::error::Result<crate::Packument> {
        todo!()
    }

    async fn tarball(
        &mut self,
        _pkg: &crate::Package,
    ) -> crate::error::Result<Box<dyn futures::AsyncRead + Unpin + Send + Sync>> {
        todo!()
    }
}
