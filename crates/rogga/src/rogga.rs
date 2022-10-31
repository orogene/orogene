use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_std::sync::Arc;
use oro_client::OroClient;
use url::Url;

pub use oro_package_spec::{PackageSpec, VersionSpec};

use crate::error::Result;
use crate::fetch::{DirFetcher, GitFetcher, NpmFetcher, PackageFetcher};
use crate::request::PackageRequest;

/// Build a new Rogga instance with specified options.
#[derive(Default)]
pub struct RoggaOpts {
    cache: Option<PathBuf>,
    registries: HashMap<String, Url>,
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

    pub fn add_registry(mut self, scope: impl AsRef<str>, registry: Url) -> Self {
        self.registries.insert(scope.as_ref().into(), registry);
        self
    }

    pub fn use_corgi(mut self, use_corgi: bool) -> Self {
        self.use_corgi = Some(use_corgi);
        self
    }

    pub fn build(self) -> Rogga {
        let client = OroClient::new("https://registry.npmjs.org".parse().unwrap());
        let use_corgi = self.use_corgi.unwrap_or(false);
        Rogga {
            // cache: self.cache,
            npm_fetcher: Arc::new(NpmFetcher::new(client.clone(), use_corgi, self.registries)),
            dir_fetcher: Arc::new(DirFetcher::new()),
            git_fetcher: Arc::new(GitFetcher::new(client)),
        }
    }
}

/// Toplevel client for making package requests.
pub struct Rogga {
    // cache: Option<PathBuf>,
    npm_fetcher: Arc<dyn PackageFetcher>,
    dir_fetcher: Arc<dyn PackageFetcher>,
    git_fetcher: Arc<dyn PackageFetcher>,
}

impl Default for Rogga {
    fn default() -> Self {
        RoggaOpts::new().build()
    }
}

impl Rogga {
    /// Creates a new Rogga instance.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a PackageRequest from a plain string spec, i.e. `foo@1.2.3`.
    pub async fn arg_request(
        &self,
        arg: impl AsRef<str>,
        base_dir: impl AsRef<Path>,
    ) -> Result<PackageRequest> {
        let spec = arg.as_ref().parse()?;
        let fetcher = self.pick_fetcher(&spec);
        let name = fetcher.name(&spec, base_dir.as_ref()).await?;
        Ok(PackageRequest {
            name,
            spec,
            fetcher,
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
        let fetcher = self.pick_fetcher(&spec);
        Ok(PackageRequest {
            name: name.as_ref().into(),
            spec,
            fetcher,
            base_dir: base_dir.as_ref().into(),
        })
    }

    fn pick_fetcher(&self, arg: &PackageSpec) -> Arc<dyn PackageFetcher> {
        use PackageSpec::*;
        match *arg {
            Dir { .. } => self.dir_fetcher.clone(),
            Alias { ref spec, .. } => self.pick_fetcher(spec),
            Npm { .. } => self.npm_fetcher.clone(),
            Git(..) => self.git_fetcher.clone(),
        }
    }
}
