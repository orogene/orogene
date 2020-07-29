use std::path::{Path, PathBuf};

use async_std::sync::{Arc, Mutex, RwLock};
use oro_client::OroClient;

pub use package_arg::PackageArg;

pub mod cache;
mod error;
mod fetch;
mod integrity;
mod package;
mod packument;

pub use error::Error;
use error::Result;
use fetch::{DirFetcher, PackageFetcher, RegistryFetcher};
pub use package::*;
pub use packument::*;

pub struct Rogga {
    client: Arc<Mutex<OroClient>>,
    cache: Option<PathBuf>,
}

impl Rogga {
    pub fn new<T: AsRef<str>>(registry: T) -> Self {
        Self {
            client: Arc::new(Mutex::new(OroClient::new(registry.as_ref()))),
            cache: None,
        }
    }

    pub fn set_cache<T: AsRef<Path>>(&mut self, cache: Option<T>) {
        self.cache = cache.map(|s| s.as_ref().into());
    }

    /// Creates a PackageRequest from a plain string spec, i.e. `foo@1.2.3`.
    pub fn arg_request<T: AsRef<str>>(&self, arg: T) -> Result<PackageRequest> {
        let spec = PackageArg::from_string(arg.as_ref())?;
        let fetcher = self.pick_fetcher(&spec);
        Ok(PackageRequest {
            name: RwLock::new(None),
            spec,
            fetcher,
        })
    }

    /// Creates a PackageRequest from a two-part dependency declaration, such
    /// as `dependencies` entries in a `package.json`.
    pub fn dep_request<T: AsRef<str>, U: AsRef<str>>(
        &self,
        name: T,
        spec: U,
    ) -> Result<PackageRequest> {
        let spec = PackageArg::resolve(name.as_ref(), spec.as_ref())?;
        let fetcher = self.pick_fetcher(&spec);
        Ok(PackageRequest {
            name: RwLock::new(Some(name.as_ref().into())),
            spec,
            fetcher,
        })
    }

    fn pick_fetcher(&self, arg: &PackageArg) -> RwLock<Box<dyn PackageFetcher>> {
        use PackageArg::*;
        match *arg {
            Dir { .. } => RwLock::new(Box::new(DirFetcher::new())),
            Alias { ref package, .. } => self.pick_fetcher(package),
            Npm { .. } => RwLock::new(Box::new(RegistryFetcher::new(self.client.clone()))),
        }
    }
}
