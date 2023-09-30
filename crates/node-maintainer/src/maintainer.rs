use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use async_std::fs;
use nassun::client::{Nassun, NassunOpts};
use nassun::package::Package;
use oro_common::CorgiManifest;
use unicase::UniCase;
use url::Url;

#[cfg(not(target_arch = "wasm32"))]
use crate::error::IoContext;
use crate::error::NodeMaintainerError;
use crate::graph::{Graph, Node};
use crate::linkers::Linker;
#[cfg(not(target_arch = "wasm32"))]
use crate::linkers::LinkerOptions;
use crate::resolver::Resolver;
use crate::{IntoKdl, Lockfile};

pub const DEFAULT_CONCURRENCY: usize = 50;
pub const DEFAULT_SCRIPT_CONCURRENCY: usize = 6;

#[cfg(not(target_arch = "wasm32"))]
pub const META_FILE_NAME: &str = ".orogene-meta.kdl";
#[cfg(not(target_arch = "wasm32"))]
pub const STORE_DIR_NAME: &str = ".oro-store";

pub type ProgressAdded = Arc<dyn Fn() + Send + Sync>;
pub type ProgressHandler = Arc<dyn Fn(&Package) + Send + Sync>;
pub type PruneProgress = Arc<dyn Fn(&Path) + Send + Sync>;
pub type ScriptStartHandler = Arc<dyn Fn(&Package, &str) + Send + Sync>;
pub type ScriptLineHandler = Arc<dyn Fn(&str) + Send + Sync>;

#[derive(Clone)]
pub struct NodeMaintainerOptions {
    nassun_opts: NassunOpts,
    nassun: Option<Nassun>,
    concurrency: usize,
    locked: bool,
    kdl_lock: Option<Lockfile>,
    npm_lock: Option<Lockfile>,

    #[allow(dead_code)]
    hoisted: bool,
    #[allow(dead_code)]
    script_concurrency: usize,
    #[allow(dead_code)]
    cache: Option<PathBuf>,
    #[allow(dead_code)]
    prefer_copy: bool,
    #[allow(dead_code)]
    validate: bool,
    #[allow(dead_code)]
    root: Option<PathBuf>,

    // Intended for progress bars
    on_resolution_added: Option<ProgressAdded>,
    on_resolve_progress: Option<ProgressHandler>,
    #[allow(dead_code)]
    on_prune_progress: Option<PruneProgress>,
    #[allow(dead_code)]
    on_extract_progress: Option<ProgressHandler>,
    #[allow(dead_code)]
    on_script_start: Option<ScriptStartHandler>,
    #[allow(dead_code)]
    on_script_line: Option<ScriptLineHandler>,
}

impl NodeMaintainerOptions {
    /// Create a new builder for NodeMaintainer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the cache location that NodeMaintainer will use.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn cache(mut self, cache: impl AsRef<Path>) -> Self {
        self.nassun_opts = self.nassun_opts.cache(PathBuf::from(cache.as_ref()));
        self.cache = Some(PathBuf::from(cache.as_ref()));
        self
    }

    /// Controls number of concurrent operations during various apply steps
    /// (resolution fetches, extractions, etc). Tuning this might help reduce
    /// memory usage.
    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    /// Make the resolver error if the newly-resolved tree would defer from
    /// an existing lockfile.
    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Controls number of concurrent script executions while running
    /// `run_script`. This option is separate from `concurrency` because
    /// executing concurrent scripts is a much heavier operation.
    pub fn script_concurrency(mut self, concurrency: usize) -> Self {
        self.script_concurrency = concurrency;
        self
    }

    /// Configure the KDL lockfile that NodeMaintainer will use.
    ///
    /// If this option is not specified, NodeMaintainer will try to read the
    /// lockfile from `<root>/package-lock.kdl`.
    pub fn kdl_lock(mut self, kdl_lock: impl IntoKdl) -> Result<Self, NodeMaintainerError> {
        let lock = Lockfile::from_kdl(kdl_lock)?;
        self.kdl_lock = Some(lock);
        Ok(self)
    }

    /// Configure the NPM lockfile that NodeMaintainer will use.
    ///
    /// If this option is not specified, NodeMaintainer will try to read the
    /// lockfile from `<root>/package-lock.json`.
    pub fn npm_lock(mut self, npm_lock: impl AsRef<str>) -> Result<Self, NodeMaintainerError> {
        let lock = Lockfile::from_npm(npm_lock)?;
        self.npm_lock = Some(lock);
        Ok(self)
    }

    /// Registry used for unscoped packages.
    ///
    /// Defaults to https://registry.npmjs.org.
    pub fn registry(mut self, registry: Url) -> Self {
        self.nassun_opts = self.nassun_opts.registry(registry);
        self
    }

    /// Registry to use for a given `@scope`. That is, what registry to use
    /// when looking up a package like `@foo/pkg`. This option can be provided
    /// multiple times.
    pub fn scope_registry(mut self, scope: impl AsRef<str>, registry: Url) -> Self {
        self.nassun_opts = self.nassun_opts.scope_registry(scope, registry);
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
        self.nassun_opts = self.nassun_opts.basic_auth(registry, username, password);
        self
    }

    /// Sets bearer token credentials for a registry.
    pub fn token_auth(mut self, registry: Url, token: impl AsRef<str>) -> Self {
        self.nassun_opts = self.nassun_opts.token_auth(registry, token.as_ref());
        self
    }

    /// Sets the legacy, pre-encoded auth token for a registry.
    pub fn legacy_auth(mut self, registry: Url, legacy_auth_token: impl AsRef<str>) -> Self {
        self.nassun_opts = self
            .nassun_opts
            .legacy_auth(registry, legacy_auth_token.as_ref());
        self
    }

    /// Root directory of the project.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn root(mut self, path: impl AsRef<Path>) -> Self {
        self.nassun_opts = self.nassun_opts.base_dir(path.as_ref());
        self.root = Some(PathBuf::from(path.as_ref()));
        self
    }

    /// Default dist-tag to use when resolving package versions.
    pub fn default_tag(mut self, tag: impl AsRef<str>) -> Self {
        self.nassun_opts = self.nassun_opts.default_tag(tag);
        self
    }

    /// Provide a pre-configured Nassun instance. Using this option will
    /// disable all other nassun-related configurations.
    pub fn nassun(mut self, nassun: Nassun) -> Self {
        self.nassun = Some(nassun);
        self
    }

    /// When extracting packages, prefer to copy files instead of linking
    /// them.
    ///
    /// This option has no effect if hard linking fails (for example, if the
    /// cache is on a different drive), or if the project is on a filesystem
    /// that supports Copy-on-Write (zfs, btrfs, APFS (macOS), etc).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn prefer_copy(mut self, prefer_copy: bool) -> Self {
        self.prefer_copy = prefer_copy;
        self
    }

    /// Use the hoisted installation mode, where all dependencies and their
    /// transitive dependencies are installed as high up in the `node_modules`
    /// tree as possible. This can potentially mean that packages have access
    /// to dependencies they did not specify in their package.json, but it
    /// might be useful for compatibility.
    pub fn hoisted(mut self, hoisted: bool) -> Self {
        self.hoisted = hoisted;
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn proxy(mut self, proxy: bool) -> Self {
        self.nassun_opts = self.nassun_opts.proxy(proxy);
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn proxy_url(mut self, proxy_url: impl AsRef<str>) -> Result<Self, NodeMaintainerError> {
        self.nassun_opts = self.nassun_opts.proxy_url(proxy_url.as_ref())?;
        Ok(self)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn no_proxy_domain(mut self, no_proxy_domain: impl AsRef<str>) -> Self {
        self.nassun_opts = self.nassun_opts.no_proxy_domain(no_proxy_domain.as_ref());
        self
    }

    pub fn on_resolution_added<F>(mut self, f: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_resolution_added = Some(Arc::new(f));
        self
    }

    pub fn on_resolve_progress<F>(mut self, f: F) -> Self
    where
        F: Fn(&Package) + Send + Sync + 'static,
    {
        self.on_resolve_progress = Some(Arc::new(f));
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn on_prune_progress<F>(mut self, f: F) -> Self
    where
        F: Fn(&Path) + Send + Sync + 'static,
    {
        self.on_prune_progress = Some(Arc::new(f));
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn on_extract_progress<F>(mut self, f: F) -> Self
    where
        F: Fn(&Package) + Send + Sync + 'static,
    {
        self.on_extract_progress = Some(Arc::new(f));
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn on_script_start<F>(mut self, f: F) -> Self
    where
        F: Fn(&Package, &str) + Send + Sync + 'static,
    {
        self.on_script_start = Some(Arc::new(f));
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn on_script_line<F>(mut self, f: F) -> Self
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.on_script_line = Some(Arc::new(f));
        self
    }

    async fn get_lockfile(&self) -> Result<Option<Lockfile>, NodeMaintainerError> {
        if let Some(kdl_lock) = &self.kdl_lock {
            return Ok(Some(kdl_lock.clone()));
        }
        if let Some(npm_lock) = &self.npm_lock {
            return Ok(Some(npm_lock.clone()));
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(root) = &self.root {
            let kdl_lock = root.join("package-lock.kdl");
            if kdl_lock.exists() {
                match async_std::fs::read_to_string(&kdl_lock)
                    .await
                    .io_context(|| format!("Failed to read {}", kdl_lock.display()))
                    .and_then(Lockfile::from_kdl)
                {
                    Ok(lock) => return Ok(Some(lock)),
                    Err(e) => tracing::debug!("Failed to parse existing package-lock.kdl: {}", e),
                }
            }
            let npm_lock = root.join("package-lock.json");
            if npm_lock.exists() {
                match async_std::fs::read_to_string(&npm_lock)
                    .await
                    .io_context(|| format!("Failed to read {}", npm_lock.display()))
                    .and_then(Lockfile::from_npm)
                {
                    Ok(lock) => return Ok(Some(lock)),
                    Err(e) => tracing::debug!("Failed to parse existing package-lock.json: {}", e),
                }
            }
            let npm_lock = root.join("npm-shrinkwrap.json");
            if npm_lock.exists() {
                match async_std::fs::read_to_string(&npm_lock)
                    .await
                    .io_context(|| format!("Failed to read {}", npm_lock.display()))
                    .and_then(Lockfile::from_npm)
                {
                    Ok(lock) => return Ok(Some(lock)),
                    Err(e) => {
                        tracing::debug!("Failed to parse existing npm-shrinkwrap.json: {}", e)
                    }
                }
            }
        }
        Ok(None)
    }

    /// Resolves a [`NodeMaintainer`] using an existing [`CorgiManifest`].
    pub async fn resolve_manifest(
        self,
        root: CorgiManifest,
    ) -> Result<NodeMaintainer, NodeMaintainerError> {
        let lockfile = self.get_lockfile().await?;
        let nassun = self.nassun.unwrap_or_else(|| self.nassun_opts.build());
        let root_pkg = Nassun::dummy_from_manifest(root.clone());
        let proj_root = self.root.unwrap_or_else(|| PathBuf::from("."));
        let mut resolver = Resolver {
            nassun,
            graph: Default::default(),
            concurrency: self.concurrency,
            locked: self.locked,
            root: &proj_root,
            actual_tree: None,
            on_resolution_added: self.on_resolution_added,
            on_resolve_progress: self.on_resolve_progress,
        };
        let node = resolver.graph.inner.add_node(Node::new(
            UniCase::new("".to_string()),
            root_pkg,
            root,
            true,
        )?);
        resolver.graph[node].root = node;
        let (graph, _actual_tree) = resolver.run_resolver(lockfile).await?;
        #[cfg(not(target_arch = "wasm32"))]
        let linker_opts = LinkerOptions {
            actual_tree: _actual_tree,
            concurrency: self.concurrency,
            script_concurrency: self.script_concurrency,
            cache: self.cache,
            prefer_copy: self.prefer_copy,
            root: proj_root,
            on_prune_progress: self.on_prune_progress,
            on_extract_progress: self.on_extract_progress,
            on_script_start: self.on_script_start,
            on_script_line: self.on_script_line,
        };
        let nm = NodeMaintainer {
            graph,
            #[cfg(target_arch = "wasm32")]
            linker: Linker::null(),
            #[cfg(not(target_arch = "wasm32"))]
            linker: if self.hoisted {
                Linker::hoisted(linker_opts)
            } else {
                Linker::isolated(linker_opts)
            },
        };
        #[cfg(debug_assertions)]
        nm.graph.validate()?;
        Ok(nm)
    }

    /// Resolves a [`NodeMaintainer`] using a particular package spec (for
    /// example, `foo@1.2.3` or `./root`) as its "root" package.
    pub async fn resolve_spec(
        self,
        root_spec: impl AsRef<str>,
    ) -> Result<NodeMaintainer, NodeMaintainerError> {
        let lockfile = self.get_lockfile().await?;
        let nassun = self.nassun_opts.build();
        let root_pkg = nassun.resolve(root_spec).await?;
        let proj_root = self.root.unwrap_or_else(|| PathBuf::from("."));
        let mut resolver = Resolver {
            nassun,
            graph: Default::default(),
            concurrency: self.concurrency,
            locked: self.locked,
            root: &proj_root,
            actual_tree: None,
            on_resolution_added: self.on_resolution_added,
            on_resolve_progress: self.on_resolve_progress,
        };
        let corgi = root_pkg.corgi_metadata().await?.manifest;
        let node = resolver.graph.inner.add_node(Node::new(
            UniCase::new("".to_string()),
            root_pkg,
            corgi,
            true,
        )?);
        resolver.graph[node].root = node;
        let (graph, _actual_tree) = resolver.run_resolver(lockfile).await?;
        #[cfg(not(target_arch = "wasm32"))]
        let linker_opts = LinkerOptions {
            actual_tree: _actual_tree,
            concurrency: self.concurrency,
            script_concurrency: self.script_concurrency,
            cache: self.cache,
            prefer_copy: self.prefer_copy,
            root: proj_root,
            on_prune_progress: self.on_prune_progress,
            on_extract_progress: self.on_extract_progress,
            on_script_start: self.on_script_start,
            on_script_line: self.on_script_line,
        };
        let nm = NodeMaintainer {
            graph,
            #[cfg(target_arch = "wasm32")]
            linker: Linker::null(),
            #[cfg(not(target_arch = "wasm32"))]
            linker: if self.hoisted {
                Linker::hoisted(linker_opts)
            } else {
                Linker::isolated(linker_opts)
            },
        };
        #[cfg(debug_assertions)]
        nm.graph.validate()?;
        Ok(nm)
    }
}

impl Default for NodeMaintainerOptions {
    fn default() -> Self {
        NodeMaintainerOptions {
            nassun_opts: Default::default(),
            nassun: None,
            concurrency: DEFAULT_CONCURRENCY,
            kdl_lock: None,
            npm_lock: None,
            locked: false,
            script_concurrency: DEFAULT_SCRIPT_CONCURRENCY,
            cache: None,
            hoisted: false,
            prefer_copy: false,
            validate: false,
            root: None,
            on_resolution_added: None,
            on_resolve_progress: None,
            on_prune_progress: None,
            on_extract_progress: None,
            on_script_start: None,
            on_script_line: None,
        }
    }
}

/// Resolves and manages `node_modules` for a given project.
pub struct NodeMaintainer {
    pub(crate) graph: Graph,
    #[allow(dead_code)]
    linker: Linker,
}

impl NodeMaintainer {
    /// Create a new [`NodeMaintainerOptions`] builder to use toconfigure a
    /// [`NodeMaintainer`].
    pub fn builder() -> NodeMaintainerOptions {
        NodeMaintainerOptions::new()
    }

    /// Resolves a [`NodeMaintainer`] using an existing [`CorgiManifest`].
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn resolve_manifest(
        root: CorgiManifest,
    ) -> Result<NodeMaintainer, NodeMaintainerError> {
        Self::builder().resolve_manifest(root).await
    }

    /// Resolves a [`NodeMaintainer`] using a particular package spec (for
    /// example, `foo@1.2.3` or `./root`) as its "root" package.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn resolve_spec(
        root_spec: impl AsRef<str>,
    ) -> Result<NodeMaintainer, NodeMaintainerError> {
        Self::builder().resolve_spec(root_spec).await
    }

    /// Writes the contents of a `package-lock.kdl` file to the file path.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn write_lockfile(&self, path: impl AsRef<Path>) -> Result<(), NodeMaintainerError> {
        let path = path.as_ref();
        fs::write(path, self.graph.to_kdl()?.to_string())
            .await
            .io_context(|| format!("Failed to write lockfile to {}", path.display()))?;
        Ok(())
    }

    /// Returns a [`crate::Lockfile`] representation of the current resolved graph.
    pub fn to_lockfile(&self) -> Result<crate::Lockfile, NodeMaintainerError> {
        self.graph.to_lockfile()
    }

    /// Returns a [`kdl::KdlDocument`] representation of the current resolved graph.
    pub fn to_kdl(&self) -> Result<kdl::KdlDocument, NodeMaintainerError> {
        self.graph.to_kdl()
    }

    /// Returns a [`Package`] for the given package spec, if it is present in
    /// the dependency tree. The path should be relative to the root of the
    /// project, and can optionally start with `"node_modules/"`.
    pub fn package_at_path(&self, path: &Path) -> Option<Package> {
        self.graph.package_at_path(path)
    }

    /// Number of unique packages in the dependency tree.
    pub fn package_count(&self) -> usize {
        self.graph.inner.node_count()
    }

    /// Scans the `node_modules` directory and removes any extraneous files or
    /// directories, including previously-installed packages that are no
    /// longer valid.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn prune(&self) -> Result<usize, NodeMaintainerError> {
        self.linker.prune(&self.graph).await
    }

    /// Extracts the `node_modules/` directory to the project root,
    /// downloading packages as needed. Whether this method creates files or
    /// hard links depends on the current filesystem and the `cache` and
    /// `prefer_copy` options.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn extract(&self) -> Result<usize, NodeMaintainerError> {
        self.linker.extract(&self.graph).await
    }

    /// Runs the `preinstall`, `install`, and `postinstall` lifecycle scripts,
    /// as well as linking the package bins as needed.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn rebuild(&self, ignore_scripts: bool) -> Result<(), NodeMaintainerError> {
        self.linker.rebuild(&self.graph, ignore_scripts).await
    }
}
