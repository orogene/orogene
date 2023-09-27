use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_std::sync::Arc;
use oro_client::{OroClient, OroClientBuilder};
use oro_common::{CorgiManifest, CorgiPackument, CorgiVersionMetadata, Packument, VersionMetadata};
use url::Url;

pub use oro_package_spec::{PackageSpec, VersionSpec};

use crate::entries::Entries;
use crate::error::Result;
#[cfg(not(target_arch = "wasm32"))]
use crate::fetch::DirFetcher;
#[cfg(not(target_arch = "wasm32"))]
use crate::fetch::GitFetcher;
use crate::fetch::{DummyFetcher, NpmFetcher, PackageFetcher};
use crate::package::Package;
use crate::resolver::{PackageResolution, PackageResolver};
use crate::tarball::Tarball;

/// Build a new Nassun instance with specified options.
#[derive(Clone, Debug, Default)]
pub struct NassunOpts {
    client_builder: OroClientBuilder,
    client: Option<OroClient>,
    #[cfg(not(target_arch = "wasm32"))]
    cache: Option<PathBuf>,
    base_dir: Option<PathBuf>,
    default_tag: Option<String>,
    registries: HashMap<Option<String>, Url>,
    memoize_metadata: bool,
}

impl NassunOpts {
    pub fn new() -> Self {
        Default::default()
    }

    /// A preconfigured [`OroClient`] to use for requests. Providing this will
    /// override all other client-related options.
    pub fn client(mut self, client: OroClient) -> Self {
        self.client = Some(client);
        self
    }

    /// Cache directory to use for requests.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn cache(mut self, cache: impl AsRef<Path>) -> Self {
        self.cache = Some(PathBuf::from(cache.as_ref()));
        self.client_builder = self.client_builder.cache(cache.as_ref());
        self
    }

    /// Sets the default registry for requests.
    pub fn registry(mut self, registry: Url) -> Self {
        self.client_builder = self.client_builder.registry(registry.clone());
        self.registries.insert(None, registry);
        self
    }

    /// Adds a registry to use for a specific scope.
    pub fn scope_registry(mut self, scope: impl AsRef<str>, registry: Url) -> Self {
        let scope = scope.as_ref();
        self.registries.insert(
            Some(scope.strip_prefix('@').unwrap_or(scope).to_string()),
            registry,
        );
        self
    }

    /// Sets basic auth credentials for a registry.
    pub fn basic_auth(
        mut self,
        registry: Url,
        username: impl AsRef<str>,
        password: Option<impl AsRef<str>>,
    ) -> Self {
        let username = username.as_ref();
        let password = password.map(|p| p.as_ref().to_string());
        self.client_builder =
            self.client_builder
                .basic_auth(registry, username.to_string(), password);
        self
    }

    /// Sets bearer token credentials for a registry.
    pub fn token_auth(mut self, registry: Url, token: impl AsRef<str>) -> Self {
        self.client_builder = self
            .client_builder
            .token_auth(registry, token.as_ref().to_string());
        self
    }

    /// Sets the legacy, pre-encoded auth token for a registry.
    pub fn legacy_auth(mut self, registry: Url, legacy_auth_token: impl AsRef<str>) -> Self {
        self.client_builder = self
            .client_builder
            .legacy_auth(registry, legacy_auth_token.as_ref().to_string());
        self
    }

    /// Base directory to use for resolving relative paths. Defaults to `"."`.
    pub fn base_dir(mut self, base_dir: impl AsRef<Path>) -> Self {
        self.base_dir = Some(PathBuf::from(base_dir.as_ref()));
        self
    }

    /// Default tag to use when resolving package versions. Defaults to `latest`.
    pub fn default_tag(mut self, default_tag: impl AsRef<str>) -> Self {
        self.default_tag = Some(default_tag.as_ref().into());
        self
    }

    /// Whether to memoize package metadata. This will keep any processed
    /// packuments in memory for the lifetime of this `Nassun` instance.
    /// Setting this to `true` may increase performance when fetching many
    /// packages, at the cost of significant additional memory usage.
    pub fn memoize_metadata(mut self, memoize: bool) -> Self {
        self.memoize_metadata = memoize;
        self
    }

    /// Number of times to retry failed requests.
    pub fn retries(mut self, retries: u32) -> Self {
        self.client_builder = self.client_builder.retries(retries);
        self
    }

    /// Whether to use a proxy for requests.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn proxy(mut self, proxy: bool) -> Self {
        self.client_builder = self.client_builder.proxy(proxy);
        self
    }

    /// Proxy URL to use for requests. If `no_proxy_domain` is needed, it must
    /// be called before this method.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn proxy_url(mut self, proxy_url: impl AsRef<str>) -> Result<Self> {
        self.client_builder = self.client_builder.proxy_url(proxy_url.as_ref())?;
        Ok(self)
    }

    /// Sets the NO_PROXY domain.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn no_proxy_domain(mut self, no_proxy_domain: impl AsRef<str>) -> Self {
        self.client_builder = self
            .client_builder
            .no_proxy_domain(no_proxy_domain.as_ref());
        self
    }

    /// Build a new Nassun instance from this options object.
    pub fn build(self) -> Nassun {
        #[cfg(not(target_arch = "wasm32"))]
        let cache = if let Some(cache) = self.cache {
            Arc::new(Some(cache))
        } else {
            Arc::new(None)
        };
        let client = self.client.unwrap_or_else(|| self.client_builder.build());
        Nassun {
            #[cfg(not(target_arch = "wasm32"))]
            cache,
            #[cfg(target_arch = "wasm32")]
            cache: Arc::new(None),
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
                self.memoize_metadata,
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
    cache: Arc<Option<PathBuf>>,
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

    /// Resolve a string spec (e.g. `foo@^1.2.3`, `github:foo/bar`, etc), to a
    /// [`Package`] that can be used for further operations.
    pub async fn resolve(&self, spec: impl AsRef<str>) -> Result<Package> {
        let spec = spec.as_ref().parse()?;
        self.resolve_spec(spec).await
    }

    /// Resolve a spec (e.g. `foo@^1.2.3`, `github:foo/bar`, etc), to a
    /// [`Package`] that can be used for further operations.
    pub async fn resolve_spec(&self, spec: PackageSpec) -> Result<Package> {
        let fetcher = self.pick_fetcher(&spec);
        let name = fetcher.name(&spec, &self.resolver.base_dir).await?;
        self.resolver
            .resolve(name, spec, fetcher, self.cache.clone())
            .await
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
        self.resolver
            .resolve_from(name, from, resolved, fetcher, self.cache.clone())
    }

    /// Creates a "resolved" package from a plain [`oro_common::Manifest`].
    /// This is useful for, say, creating dummy packages for top-level
    /// projects.
    pub fn dummy_from_manifest(manifest: CorgiManifest) -> Package {
        Package {
            cache: Arc::new(None),
            from: PackageSpec::Dir {
                path: PathBuf::from("."),
            },
            name: manifest.name.clone().unwrap_or_else(|| "dummy".to_string()),
            resolved: PackageResolution::Dir {
                name: manifest.name.clone().unwrap_or_else(|| "dummy".to_string()),
                path: PathBuf::from("."),
            },
            base_dir: PathBuf::from("."),
            fetcher: Arc::new(DummyFetcher(manifest)),
        }
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
