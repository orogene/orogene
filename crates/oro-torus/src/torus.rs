use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use oro_api_client::ApiClient;
use url::Url;

pub use oro_package_spec::{PackageSpec, VersionSpec};

use crate::error::TorusError;
use crate::fetch::{DirFetcher, GitFetcher, NpmFetcher, PackageFetcher};
use crate::registry::Registry;
use crate::request::PackageRequest;

/// Build a new Torus instance with specified options.
#[derive(Default)]
pub struct TorusOpts {
    cache: Option<PathBuf>,
    registries: HashMap<String, Registry>,
    use_corgi: Option<bool>,
}

impl TorusOpts {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn cache(mut self, cache: impl AsRef<Path>) -> Self {
        self.cache = Some(PathBuf::from(cache.as_ref()));
        self
    }

    pub fn registry_url(mut self, url: Url) -> Self {
        self.registries.insert(
            "".to_string(),
            Registry {
                scope: None,
                url,
                auth: None,
            },
        );
        self
    }

    pub fn registry(mut self, registry_config: Registry) -> Self {
        let scope = if let Some(mut scope) = registry_config.scope.clone() {
            if scope.get(0..1) != Some("@") {
                scope.insert(0, '@');
            }
            scope
        } else {
            "".to_string()
        };
        self.registries.insert(scope, registry_config);
        self
    }

    pub fn use_corgi(mut self, use_corgi: bool) -> Self {
        self.use_corgi = Some(use_corgi);
        self
    }

    pub fn build(self) -> Torus {
        let client = ApiClient::new();
        let use_corgi = self.use_corgi.unwrap_or(false);
        Torus {
            // cache: self.cache,
            npm_fetcher: Arc::new(NpmFetcher::new(client.clone(), use_corgi, self.registries)),
            dir_fetcher: Arc::new(DirFetcher::new()),
            git_fetcher: Arc::new(GitFetcher::new(client)),
        }
    }
}

/// Toplevel client for making package requests.
#[derive(Debug, Clone)]
pub struct Torus {
    // cache: Option<PathBuf>,
    npm_fetcher: Arc<dyn PackageFetcher>,
    dir_fetcher: Arc<dyn PackageFetcher>,
    git_fetcher: Arc<dyn PackageFetcher>,
}

impl Default for Torus {
    fn default() -> Self {
        TorusOpts::new().build()
    }
}

impl Torus {
    /// Creates a new Torus instance.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a [PackageRequest] from a plain string spec, i.e. `foo@1.2.3`.
    pub async fn arg_request(
        &self,
        arg: impl AsRef<str>,
        base_dir: impl AsRef<Path>,
    ) -> Result<PackageRequest, TorusError> {
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

    /// Creates a [PackageRequest] from a two-part dependency declaration, such
    /// as `dependencies` entries in a `package.json`.
    pub fn dep_request(
        &self,
        name: impl AsRef<str>,
        spec: impl AsRef<str>,
        base_dir: impl AsRef<Path>,
    ) -> Result<PackageRequest, TorusError> {
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
