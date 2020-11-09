use std::path::{Path, PathBuf};

use async_std::sync::{Arc, Mutex};
use oro_client::OroClient;

pub use oro_package_spec::{PackageSpec, VersionSpec};

use crate::error::Result;
use crate::fetch::PackageFetcherPool;
use crate::request::PackageRequest;

/// Build a new Rogga instance with specified options.
#[derive(Default)]
pub struct RoggaOpts {
    cache: Option<PathBuf>,
    registry: Option<String>,
    use_corgi: Option<bool>,
    pool_capacity: Option<usize>,
}

impl RoggaOpts {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn cache(mut self, cache: impl AsRef<Path>) -> Self {
        self.cache = Some(PathBuf::from(cache.as_ref()));
        self
    }

    pub fn registry(mut self, registry: impl AsRef<str>) -> Self {
        self.registry = Some(String::from(registry.as_ref()));
        self
    }

    pub fn use_corgi(mut self, use_corgi: bool) -> Self {
        self.use_corgi = Some(use_corgi);
        self
    }

    pub fn pool_capacity(mut self, capacity: usize) -> Self {
        self.pool_capacity = Some(capacity);
        self
    }

    pub fn build(self) -> Rogga {
        let reg = self
            .registry
            .unwrap_or_else(|| "https://registry.npmjs.org".into());
        let client = Arc::new(Mutex::new(OroClient::new(reg)));
        let use_corgi = self.use_corgi.unwrap_or(false);
        let capacity = self.pool_capacity.unwrap_or(20);
        Rogga {
            // cache: self.cache,
            registry_pool: PackageFetcherPool::new_registry(client.clone(), use_corgi, capacity),
            dir_pool: PackageFetcherPool::new_dir(capacity),
            git_pool: PackageFetcherPool::new_git(client, capacity),
        }
    }
}

/// Toplevel client for making package requests.
pub struct Rogga {
    // cache: Option<PathBuf>,
    registry_pool: PackageFetcherPool,
    dir_pool: PackageFetcherPool,
    git_pool: PackageFetcherPool,
}

impl Rogga {
    /// Creates a new Rogga instance.
    pub fn new(registry: impl AsRef<str>) -> Self {
        RoggaOpts::new().registry(registry.as_ref()).build()
    }

    /// Creates a PackageRequest from a plain string spec, i.e. `foo@1.2.3`.
    pub async fn arg_request(
        &self,
        arg: impl AsRef<str>,
        base_dir: impl AsRef<Path>,
    ) -> Result<PackageRequest> {
        let spec = arg.as_ref().parse()?;
        let pool = self.pick_pool(&spec);
        let name = pool.get().await.name(&spec, base_dir.as_ref()).await?;
        Ok(PackageRequest {
            name,
            spec,
            fetcher_pool: pool,
            base_dir: base_dir.as_ref().into(),
        })
    }

    /// Creates a PackageRequest from a two-part dependency declaration, such
    /// as `dependencies` entries in a `package.json`.
    pub fn dep_request(
        &self,
        name: impl AsRef<str>,
        spec: impl AsRef<str>,
        base_dir: impl AsRef<Path>,
    ) -> Result<PackageRequest> {
        let spec = format!("{}@{}", name.as_ref(), spec.as_ref()).parse()?;
        let pool = self.pick_pool(&spec);
        Ok(PackageRequest {
            name: name.as_ref().into(),
            spec,
            fetcher_pool: pool,
            base_dir: base_dir.as_ref().into(),
        })
    }

    fn pick_pool(&self, arg: &PackageSpec) -> PackageFetcherPool {
        use PackageSpec::*;
        match *arg {
            Dir { .. } => self.dir_pool.clone(),
            Alias { ref package, .. } => self.pick_pool(package),
            Npm { .. } => self.registry_pool.clone(),
            Git(..) => self.git_pool.clone(),
        }
    }
}
