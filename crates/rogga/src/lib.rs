use std::path::{Path, PathBuf};

use async_std::sync::{Arc, Mutex, RwLock};
use oro_client::OroClient;

pub use package_spec::{PackageSpec, VersionSpec};

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

/// Build a new Rogga instance with specified options.
#[derive(Default)]
pub struct RoggaOpts {
    cache: Option<PathBuf>,
    registry: Option<String>,
    use_corgi: Option<bool>,
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

    pub fn build(self) -> Rogga {
        let reg = self
            .registry
            .unwrap_or_else(|| "https://registry.npmjs.org".into());
        Rogga {
            // cache: self.cache,
            client: Arc::new(Mutex::new(OroClient::new(reg))),
            use_corgi: self.use_corgi.unwrap_or(false),
        }
    }
}

/// Toplevel client for making package requests.
pub struct Rogga {
    client: Arc<Mutex<OroClient>>,
    // cache: Option<PathBuf>,
    use_corgi: bool,
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
        dir: impl AsRef<Path>,
    ) -> Result<PackageRequest> {
        let spec = PackageSpec::from_string(arg.as_ref(), dir.as_ref())?;
        let fetcher = self.pick_fetcher(&spec);
        let name = {
            let mut locked = fetcher.write().await;
            locked.name(&spec).await?
        };
        Ok(PackageRequest {
            name,
            spec,
            fetcher,
        })
    }

    /// Creates a PackageRequest from a two-part dependency declaration, such
    /// as `dependencies` entries in a `package.json`.
    pub fn dep_request(
        &self,
        name: impl AsRef<str>,
        spec: impl AsRef<str>,
        dir: impl AsRef<Path>,
    ) -> Result<PackageRequest> {
        let spec = PackageSpec::resolve(name.as_ref(), spec.as_ref(), dir.as_ref())?;
        let fetcher = self.pick_fetcher(&spec);
        Ok(PackageRequest {
            name: name.as_ref().into(),
            spec,
            fetcher,
        })
    }

    /// Picks a fetcher from the fetchers available in src/fetch, according to
    /// the requested PackageArg.
    fn pick_fetcher(&self, arg: &PackageSpec) -> RwLock<Box<dyn PackageFetcher>> {
        use PackageSpec::*;
        match *arg {
            Dir { .. } => RwLock::new(Box::new(DirFetcher::new())),
            Alias { ref package, .. } => self.pick_fetcher(package),
            Npm { .. } => RwLock::new(Box::new(RegistryFetcher::new(
                self.client.clone(),
                self.use_corgi,
            ))),
        }
    }
}
