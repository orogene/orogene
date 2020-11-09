use std::path::Path;

use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use deadpool::managed::{Object, Pool};
use futures::io::AsyncRead;
use oro_client::OroClient;
use oro_package_spec::PackageSpec;

use crate::error::{Error, Result};
use crate::package::Package;
use crate::packument::{Packument, VersionMetadata};

pub use dir::DirFetcherPool;
pub use git::GitFetcherPool;
pub use registry::RegistryFetcherPool;

mod dir;
mod git;
mod registry;

#[derive(Clone)]
pub struct PackageFetcherPool(Pool<Box<dyn PackageFetcher>, Error>);

impl PackageFetcherPool {
    pub async fn get(&self) -> Object<Box<dyn PackageFetcher>, Error> {
        self.0
            .get()
            .await
            .expect("All fetchers have infallible creators")
    }

    pub fn new_git(client: Arc<Mutex<OroClient>>, capacity: usize) -> Self {
        Self(Pool::new(GitFetcherPool::new(client), capacity))
    }

    pub fn new_registry(client: Arc<Mutex<OroClient>>, use_corgi: bool, capacity: usize) -> Self {
        Self(Pool::new(
            RegistryFetcherPool::new(client, use_corgi),
            capacity,
        ))
    }

    pub fn new_dir(capacity: usize) -> Self {
        Self(Pool::new(DirFetcherPool::new(), capacity))
    }
}

#[async_trait]
pub trait PackageFetcher: std::fmt::Debug + Send + Sync {
    async fn name(&mut self, spec: &PackageSpec, base_dir: &Path) -> Result<String>;
    async fn metadata(&mut self, pkg: &Package) -> Result<VersionMetadata>;
    async fn packument(&mut self, pkg: &PackageSpec, base_dir: &Path) -> Result<Packument>;
    async fn tarball(&mut self, pkg: &Package) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>>;
}
