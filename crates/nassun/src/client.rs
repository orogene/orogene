use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_std::sync::Arc;
use oro_client::OroClient;
use oro_common::{CorgiPackument, CorgiVersionMetadata, Packument, VersionMetadata};
use url::Url;

pub use oro_package_spec::{PackageSpec, VersionSpec};

use crate::error::Result;
#[cfg(not(target_arch = "wasm32"))]
use crate::fetch::DirFetcher;
#[cfg(not(target_arch = "wasm32"))]
use crate::fetch::GitFetcher;
use crate::fetch::{NpmFetcher, PackageFetcher};
use crate::package::Package;
use crate::resolver::PackageResolver;
use crate::{Entries, PackageResolution, Tarball};

/// Build a new Nassun instance with specified options.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct NassunOpts {
    cache: Option<PathBuf>,
    base_dir: Option<PathBuf>,
    default_tag: Option<String>,
    registries: HashMap<Option<String>, Url>,
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

    pub fn build(self) -> Nassun {
        let registry = self
            .registries
            .get(&None)
            .cloned()
            .unwrap_or_else(|| "https://registry.npmjs.org/".parse().unwrap());
        let client = OroClient::new(registry);
        Nassun {
            // cache: self.cache,
            resolver: PackageResolver {
                #[cfg(target_arch = "wasm32")]
                base_dir: PathBuf::from("."),
                #[cfg(not(target_arch = "wasm32"))]
                base_dir: self
                    .base_dir
                    .unwrap_or_else(|| std::env::current_dir().expect("failed to get cwd.")),
                default_tag: self.default_tag.unwrap_or_else(|| "latest".into()),
            },
            npm_fetcher: Arc::new(NpmFetcher::new(
                #[allow(clippy::redundant_clone)]
                client.clone(),
                self.registries,
            )),
            #[cfg(not(target_arch = "wasm32"))]
            dir_fetcher: Arc::new(DirFetcher::new()),
            #[cfg(not(target_arch = "wasm32"))]
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
    #[cfg(not(target_arch = "wasm32"))]
    dir_fetcher: Arc<dyn PackageFetcher>,
    #[cfg(not(target_arch = "wasm32"))]
    git_fetcher: Arc<dyn PackageFetcher>,
}

impl Default for Nassun {
    fn default() -> Self {
        NassunOpts::new().build()
    }
}

impl Nassun {
    /// Creates a new `Nassun` instance with default settings. To configure
    /// `Nassun`, use [`NassunOpts`].
    pub fn new() -> Self {
        Default::default()
    }

    /// Resolves a [`Packument`] for the given package `spec`.
    ///
    /// This uses default [`Nassun`] settings and does not cache the result.
    /// To configure `Nassun`, and/or enable more efficient caching/reuse,
    /// look at [`Package::packument` instead].
    pub async fn packument(spec: impl AsRef<str>) -> Result<Arc<Packument>> {
        Self::new().resolve(spec.as_ref()).await?.packument().await
    }

    /// Resolves a partial (corgi) version of the [`Packument`] for the given
    /// package `spec`.
    ///
    /// This uses default [`Nassun`] settings and does not cache the result.
    /// To configure `Nassun`, and/or enable more efficient caching/reuse,
    /// look at [`Package::packument` instead].
    pub async fn corgi_packument(spec: impl AsRef<str>) -> Result<Arc<CorgiPackument>> {
        Self::new()
            .resolve(spec.as_ref())
            .await?
            .corgi_packument()
            .await
    }

    /// Resolves a [`VersionMetadata`] from the given package `spec`, using
    /// the default resolution algorithm.
    ///
    /// This uses default [`Nassun`] settings and does not cache the result.
    /// To configure `Nassun`, and/or enable more efficient caching/reuse,
    /// look at [`Package::metadata` instead].
    pub async fn metadata(spec: impl AsRef<str>) -> Result<VersionMetadata> {
        Self::new().resolve(spec.as_ref()).await?.metadata().await
    }

    /// Resolves a partial (corgi) version of the [`VersionMetadata`] from the
    /// given package `spec`, using the default resolution algorithm.
    ///
    /// This uses default [`Nassun`] settings and does not cache the result.
    /// To configure `Nassun`, and/or enable more efficient caching/reuse,
    /// look at [`Package::metadata` instead].
    pub async fn corgi_metadata(spec: impl AsRef<str>) -> Result<CorgiVersionMetadata> {
        Self::new()
            .resolve(spec.as_ref())
            .await?
            .corgi_metadata()
            .await
    }

    /// Resolves a [`Tarball`] from the given package `spec`, using the
    /// default resolution algorithm. This tarball will have its data checked
    /// if the package metadata fetched includes integrity information.
    ///
    /// This uses default [`Nassun`] settings and does not cache the result.
    /// To configure `Nassun`, and/or enable more efficient caching/reuse,
    /// look at [`Package::tarball`] instead.
    pub async fn tarball(spec: impl AsRef<str>) -> Result<Tarball> {
        Self::new().resolve(spec.as_ref()).await?.tarball().await
    }

    /// Resolves [`Entries`] from the given package `spec`, using the
    /// default resolution algorithm. The source tarball will have its data
    /// checked if the package metadata fetched includes integrity
    /// information.
    ///
    /// This uses default [`Nassun`] settings and does not cache the result.
    /// To configure `Nassun`, and/or enable more efficient caching/reuse,
    /// look at [`Package::entries`] instead.
    pub async fn entries(spec: impl AsRef<str>) -> Result<Entries> {
        Self::new().resolve(spec.as_ref()).await?.entries().await
    }

    /// Resolve a spec (e.g. `foo@^1.2.3`, `github:foo/bar`, etc), to a
    /// [`Package`] that can be used for further operations.
    pub async fn resolve(&self, spec: impl AsRef<str>) -> Result<Package> {
        let spec = spec.as_ref().parse()?;
        let fetcher = self.pick_fetcher(&spec);
        let name = fetcher.name(&spec, &self.resolver.base_dir).await?;
        self.resolver.resolve(name, spec, fetcher).await
    }

    /// Resolves a package directly from a previously-calculated
    /// [`PackageResolution`]. This is meant to be a lower-level call that
    /// expects the caller to have already done any necessary parsing work on
    /// its arguments.
    pub fn resolve_from(
        &self,
        name: String,
        from: PackageSpec,
        resolved: PackageResolution,
    ) -> Package {
        let fetcher = self.pick_fetcher(&from);
        self.resolver.resolve_from(name, from, resolved, fetcher)
    }

    fn pick_fetcher(&self, arg: &PackageSpec) -> Arc<dyn PackageFetcher> {
        use PackageSpec::*;
        match *arg {
            Alias { ref spec, .. } => self.pick_fetcher(spec),
            Npm { .. } => self.npm_fetcher.clone(),
            #[cfg(not(target_arch = "wasm32"))]
            Dir { .. } => self.dir_fetcher.clone(),
            #[cfg(target_arch = "wasm32")]
            Dir { .. } => panic!(
                "Directory dependencies are not enabled. (While trying to process {})",
                arg
            ),
            #[cfg(not(target_arch = "wasm32"))]
            Git(..) => self.git_fetcher.clone(),
            #[cfg(target_arch = "wasm32")]
            Git(..) => panic!(
                "Git dependencies are not enabled. (While trying to process {})",
                arg
            ),
        }
    }
}
