use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_std::sync::Arc;
use oro_client::OroClient;
use url::Url;

pub use oro_package_spec::{PackageSpec, VersionSpec};

use crate::error::Result;
#[cfg(feature = "dir")]
use crate::fetch::DirFetcher;
#[cfg(feature = "git")]
use crate::fetch::GitFetcher;
use crate::fetch::{NpmFetcher, PackageFetcher};
use crate::request::PackageRequest;

/// Build a new Nassun instance with specified options.
#[derive(Default)]
pub struct NassunOpts {
    cache: Option<PathBuf>,
    base_dir: Option<PathBuf>,
    registries: HashMap<Option<String>, Url>,
    use_corgi: Option<bool>,
}

impl NassunOpts {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn cache(mut self, cache: impl AsRef<Path>) -> Self {
        self.cache = Some(PathBuf::from(cache.as_ref()));
        self
    }

    pub fn registry(mut self, registry: Url) -> Self {
        self.registries.insert(None, registry);
        self
    }

    pub fn scope_registry(mut self, scope: impl AsRef<str>, registry: Url) -> Self {
        self.registries
            .insert(Some(scope.as_ref().into()), registry);
        self
    }

    pub fn base_dir(mut self, base_dir: impl AsRef<Path>) -> Self {
        self.base_dir = Some(PathBuf::from(base_dir.as_ref()));
        self
    }

    pub fn use_corgi(mut self, use_corgi: bool) -> Self {
        self.use_corgi = Some(use_corgi);
        self
    }

    pub fn build(self) -> Nassun {
        let registry = self
            .registries
            .get(&None)
            .cloned()
            .unwrap_or_else(|| "https://registry.npmjs.org/".parse().unwrap());
        let client = OroClient::new(registry);
        let use_corgi = self.use_corgi.unwrap_or(true);
        Nassun {
            // cache: self.cache,
            base_dir: self
                .base_dir
                .unwrap_or_else(|| std::env::current_dir().expect("failed to get cwd.")),
            npm_fetcher: Arc::new(NpmFetcher::new(
                #[allow(clippy::redundant_clone)]
                client.clone(),
                use_corgi,
                self.registries,
            )),
            #[cfg(feature = "dir")]
            dir_fetcher: Arc::new(DirFetcher::new()),
            #[cfg(feature = "git")]
            git_fetcher: Arc::new(GitFetcher::new(client)),
        }
    }
}

/// Toplevel client for making package requests.
#[derive(Clone)]
pub struct Nassun {
    // cache: Option<PathBuf>,
    base_dir: PathBuf,
    npm_fetcher: Arc<dyn PackageFetcher>,
    #[cfg(feature = "dir")]
    dir_fetcher: Arc<dyn PackageFetcher>,
    #[cfg(feature = "git")]
    git_fetcher: Arc<dyn PackageFetcher>,
}

impl Default for Nassun {
    fn default() -> Self {
        NassunOpts::new().build()
    }
}

impl Nassun {
    /// Creates a new Nassun instance.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a PackageRequest from a plain string spec, i.e. `foo@1.2.3`.
    pub async fn arg_request(&self, arg: impl AsRef<str>) -> Result<PackageRequest> {
        let spec = arg.as_ref().parse()?;
        let fetcher = self.pick_fetcher(&spec);
        let name = fetcher.name(&spec, &self.base_dir).await?;
        Ok(PackageRequest {
            name,
            spec,
            fetcher,
            base_dir: self.base_dir.clone(),
        })
    }

    /// Creates a PackageRequest from a two-part dependency declaration, such
    /// as `dependencies` entries in a `package.json`.
    pub fn dep_request(
        &self,
        name: impl AsRef<str>,
        spec: impl AsRef<str>,
    ) -> Result<PackageRequest> {
        let spec = format!("{}@{}", name.as_ref(), spec.as_ref()).parse()?;
        let fetcher = self.pick_fetcher(&spec);
        Ok(PackageRequest {
            name: name.as_ref().into(),
            spec,
            fetcher,
            base_dir: self.base_dir.clone(),
        })
    }

    fn pick_fetcher(&self, arg: &PackageSpec) -> Arc<dyn PackageFetcher> {
        use PackageSpec::*;
        match *arg {
            Alias { ref spec, .. } => self.pick_fetcher(spec),
            Npm { .. } => self.npm_fetcher.clone(),
            #[cfg(feature = "dir")]
            Dir { .. } => self.dir_fetcher.clone(),
            #[cfg(not(feature = "dir"))]
            Dir { .. } => panic!("Directory dependencies are not enabled."),
            #[cfg(feature = "git")]
            Git(..) => self.git_fetcher.clone(),
            #[cfg(not(feature = "git"))]
            Git(..) => panic!("Git dependencies are not enabled."),
        }
    }
}
