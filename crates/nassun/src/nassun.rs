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
use crate::package::Package;
use crate::resolver::PackageResolver;

/// Build a new Nassun instance with specified options.
#[derive(Default)]
pub struct NassunOpts {
    cache: Option<PathBuf>,
    base_dir: Option<PathBuf>,
    default_tag: Option<String>,
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

    pub fn default_tag(mut self, default_tag: impl AsRef<str>) -> Self {
        self.default_tag = Some(default_tag.as_ref().into());
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
            resolver: PackageResolver {
                base_dir: self
                    .base_dir
                    .unwrap_or_else(|| std::env::current_dir().expect("failed to get cwd.")),
                default_tag: self.default_tag.unwrap_or_else(|| "latest".into()),
            },
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
    resolver: PackageResolver,
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

    /// Resolve a spec (e.g. `foo@^1.2.3`, `github:foo/bar`, etc), to a
    /// [`Package`] that can be used for further operations.
    pub async fn resolve(&self, spec: impl AsRef<str>) -> Result<Package> {
        let spec = spec.as_ref().parse()?;
        let fetcher = self.pick_fetcher(&spec);
        let name = fetcher.name(&spec, &self.resolver.base_dir).await?;
        self.resolver.resolve(name, spec, fetcher).await
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
