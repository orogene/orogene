#[cfg(not(target_arch = "wasm32"))]
use std::ffi::OsStr;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{self, AtomicUsize};
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use async_std::fs;
use futures::StreamExt;
#[cfg(not(target_arch = "wasm32"))]
use futures::TryStreamExt;
use nassun::client::{Nassun, NassunOpts};
use nassun::package::Package;
#[cfg(not(target_arch = "wasm32"))]
use oro_common::BuildManifest;
use oro_common::CorgiManifest;
#[cfg(not(target_arch = "wasm32"))]
use oro_script::OroScript;
use unicase::UniCase;
use url::Url;

use crate::error::NodeMaintainerError;
use crate::graph::{Graph, Node};
use crate::resolver::Resolver;
use crate::{IntoKdl, Lockfile};

pub const DEFAULT_CONCURRENCY: usize = 50;
pub const DEFAULT_SCRIPT_CONCURRENCY: usize = 6;
pub const META_FILE_NAME: &str = ".orogene-meta.kdl";

pub type ProgressAdded = Arc<dyn Fn() + Send + Sync>;
pub type ProgressHandler = Arc<dyn Fn(&Package) + Send + Sync>;
pub type PruneProgress = Arc<dyn Fn(&Path) + Send + Sync>;
pub type ScriptStartHandler = Arc<dyn Fn(&Package, &str) + Send + Sync>;
pub type ScriptLineHandler = Arc<dyn Fn(&str) + Send + Sync>;

#[derive(Clone)]
pub struct NodeMaintainerOptions {
    nassun_opts: NassunOpts,
    concurrency: usize,
    kdl_lock: Option<Lockfile>,
    npm_lock: Option<Lockfile>,

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
    on_prune_progress: Option<PruneProgress>,
    on_extract_progress: Option<ProgressHandler>,
    on_script_start: Option<ScriptStartHandler>,
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

    /// Controls number of concurrent operations during various restore steps
    /// (resolution fetches, extractions, etc). Tuning this might help reduce
    /// memory usage.
    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
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

    /// When extracting packages, prefer to copy files files instead of
    /// linking them.
    ///
    /// This option has no effect if hard linking fails (for example, if the
    /// cache is on a different drive), or if the project is on a filesystem
    /// that supports Copy-on-Write (zfs, btrfs, APFS (macOS), etc).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn prefer_copy(mut self, prefer_copy: bool) -> Self {
        self.prefer_copy = prefer_copy;
        self
    }

    /// When this is true, node-maintainer will validate integrity hashes for
    /// all files extracted from the cache, as well as verify that any files
    /// in the existing `node_modules` are unmodified. If verification fails,
    /// the packages will be reinstalled.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn validate(mut self, validate: bool) -> Self {
        self.validate = validate;
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
                match async_std::fs::read_to_string(kdl_lock)
                    .await
                    .map_err(NodeMaintainerError::IoError)
                    .and_then(Lockfile::from_kdl)
                {
                    Ok(lock) => return Ok(Some(lock)),
                    Err(e) => tracing::debug!("Failed to parse existing package-lock.kdl: {}", e),
                }
            }
            let npm_lock = root.join("package-lock.json");
            if npm_lock.exists() {
                match async_std::fs::read_to_string(npm_lock)
                    .await
                    .map_err(NodeMaintainerError::IoError)
                    .and_then(Lockfile::from_npm)
                {
                    Ok(lock) => return Ok(Some(lock)),
                    Err(e) => tracing::debug!("Failed to parse existing package-lock.json: {}", e),
                }
            }
            let npm_lock = root.join("npm-shrinkwrap.json");
            if npm_lock.exists() {
                match async_std::fs::read_to_string(npm_lock)
                    .await
                    .map_err(NodeMaintainerError::IoError)
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
        let nassun = self.nassun_opts.build();
        let root_pkg = Nassun::dummy_from_manifest(root.clone());
        let proj_root = self.root.unwrap_or_else(|| PathBuf::from("."));
        let mut resolver = Resolver {
            nassun,
            graph: Default::default(),
            concurrency: self.concurrency,
            root: &proj_root,
            actual_tree: None,
            on_resolution_added: self.on_resolution_added,
            on_resolve_progress: self.on_resolve_progress,
        };
        let node = resolver.graph.inner.add_node(Node::new(root_pkg, root));
        resolver.graph[node].root = node;
        let (graph, actual_tree) = resolver.run_resolver(lockfile).await?;
        let nm = NodeMaintainer {
            graph,
            actual_tree,
            concurrency: self.concurrency,
            script_concurrency: self.script_concurrency,
            cache: self.cache,
            prefer_copy: self.prefer_copy,
            validate: self.validate,
            root: proj_root,
            on_prune_progress: self.on_prune_progress,
            on_extract_progress: self.on_extract_progress,
            on_script_start: self.on_script_start,
            on_script_line: self.on_script_line,
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
            root: &proj_root,
            actual_tree: None,
            on_resolution_added: self.on_resolution_added,
            on_resolve_progress: self.on_resolve_progress,
        };
        let corgi = root_pkg.corgi_metadata().await?.manifest;
        let node = resolver.graph.inner.add_node(Node::new(root_pkg, corgi));
        resolver.graph[node].root = node;
        let (graph, actual_tree) = resolver.run_resolver(lockfile).await?;
        let nm = NodeMaintainer {
            graph,
            actual_tree,
            concurrency: self.concurrency,
            script_concurrency: self.script_concurrency,
            cache: self.cache,
            prefer_copy: self.prefer_copy,
            validate: self.validate,
            root: proj_root,
            on_prune_progress: self.on_prune_progress,
            on_extract_progress: self.on_extract_progress,
            on_script_start: self.on_script_start,
            on_script_line: self.on_script_line,
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
            concurrency: DEFAULT_CONCURRENCY,
            kdl_lock: None,
            npm_lock: None,
            script_concurrency: DEFAULT_SCRIPT_CONCURRENCY,
            cache: None,
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
    concurrency: usize,
    actual_tree: Option<Lockfile>,
    #[allow(dead_code)]
    script_concurrency: usize,
    #[allow(dead_code)]
    cache: Option<PathBuf>,
    #[allow(dead_code)]
    prefer_copy: bool,
    #[allow(dead_code)]
    validate: bool,
    #[allow(dead_code)]
    root: PathBuf,
    #[allow(dead_code)]
    on_prune_progress: Option<PruneProgress>,
    #[allow(dead_code)]
    on_extract_progress: Option<ProgressHandler>,
    #[allow(dead_code)]
    on_script_start: Option<ScriptStartHandler>,
    #[allow(dead_code)]
    on_script_line: Option<ScriptLineHandler>,
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
        fs::write(path.as_ref(), self.graph.to_kdl()?.to_string()).await?;
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
        use walkdir::WalkDir;

        let prefix = self.root.join("node_modules");

        if !prefix.exists() {
            return Ok(0);
        }

        let start = std::time::Instant::now();

        if self.actual_tree.is_none() {
            // If there's no actual tree previously calculated, we can't trust
            // *anything* inside node_modules, so everything is immediately
            // extraneous and we wipe it all. Sorry.
            let mut entries = async_std::fs::read_dir(&prefix).await?;
            while let Some(entry) = entries.next().await {
                let entry = entry?;
                if entry.file_type().await?.is_dir() {
                    async_std::fs::remove_dir_all(entry.path()).await?;
                } else {
                    async_std::fs::remove_file(entry.path()).await?;
                }
            }

            tracing::debug!("No metadata file found in node_modules/. Pruned entire node_modules/ directory in {}ms.", start.elapsed().as_micros() / 1000);

            // TODO: get an accurate count here?
            return Ok(0);
        }

        let nm_osstr = Some(std::ffi::OsStr::new("node_modules"));
        let bin_osstr = Some(std::ffi::OsStr::new(".bin"));
        let meta = prefix.join(META_FILE_NAME);
        let mut extraneous_packages = 0;
        let extraneous = &mut extraneous_packages;

        for entry in WalkDir::new(&prefix)
            .into_iter()
            .filter_entry(move |entry| {
                let entry_path = entry.path();

                if entry_path == meta {
                    // Skip the meta file
                    return false;
                }

                let file_name = entry_path.file_name();

                if file_name == nm_osstr {
                    // We don't want to skip node_modules themselves
                    return true;
                }

                if file_name == bin_osstr {
                    return false;
                }

                if file_name
                    .expect("this should have a file name")
                    .to_string_lossy()
                    .starts_with('@')
                {
                    // Let scoped packages through.
                    return true;
                }

                // See if we're looking at a package dir, presumably (or a straggler file).
                if entry_path
                    .parent()
                    .expect("this must have a parent")
                    .file_name()
                    == nm_osstr
                {
                    let entry_subpath_path = entry_path
                        .strip_prefix(&prefix)
                        .expect("this should definitely be under the prefix");
                    let entry_subpath =
                        UniCase::from(entry_subpath_path.to_string_lossy().replace('\\', "/"));

                    let actual = self
                        .actual_tree
                        .as_ref()
                        .and_then(|tree| tree.packages.get(&entry_subpath));
                    let ideal = self
                        .graph
                        .node_at_path(entry_subpath_path)
                        .and_then(|node| self.graph.node_lockfile_node(node.idx, false).ok());
                    // If the package is not in the actual tree, or it doesn't
                    // match up with what the ideal tree wants, it's
                    // extraneous. We want to return true for those so we
                    // delete them later.
                    if ideal.is_some()
                        && self
                            .actual_tree
                            .as_ref()
                            .map(|tree| tree.packages.contains_key(&entry_subpath))
                            .unwrap_or(false)
                        && actual == ideal.as_ref()
                    {
                        return false;
                    } else {
                        *extraneous += 1;
                        return true;
                    }
                }

                // We're not interested in any other files than the package dirs themselves.
                false
            })
        {
            let entry = entry?;
            let entry_path = entry.path();
            let file_name = entry_path.file_name();
            if file_name == nm_osstr
                || file_name == bin_osstr
                || file_name
                    .map(|s| s.to_string_lossy().starts_with('@'))
                    .unwrap_or(false)
            {
                continue;
            } else if entry.file_type().is_dir() {
                if let Some(pb) = &self.on_prune_progress {
                    pb(entry_path);
                }
                tracing::trace!("Pruning extraneous directory: {}", entry.path().display());
                async_std::fs::remove_dir_all(entry.path()).await?;
            } else {
                if let Some(pb) = &self.on_prune_progress {
                    pb(entry_path);
                }
                tracing::trace!("Pruning extraneous file: {}", entry.path().display());
                async_std::fs::remove_file(entry.path()).await?;
            }
        }

        if extraneous_packages == 0 {
            tracing::debug!(
                "Nothing to prune. Completed check in {}ms.",
                start.elapsed().as_micros() / 1000
            );
        } else {
            tracing::debug!(
                "Pruned {extraneous_packages} extraneous package{} in {}ms.",
                start.elapsed().as_micros() / 1000,
                if extraneous_packages == 1 { "" } else { "s" },
            );
        }
        Ok(extraneous_packages)
    }

    /// Extracts the `node_modules/` directory to the project root,
    /// downloading packages as needed. Whether this method creates files or
    /// hard links depends on the current filesystem and the `cache` and
    /// `prefer_copy` options.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn extract(&self) -> Result<usize, NodeMaintainerError> {
        tracing::debug!("Extracting node_modules/...");
        let start = std::time::Instant::now();

        let root = &self.root;
        let stream = futures::stream::iter(self.graph.inner.node_indices());
        let concurrent_count = Arc::new(AtomicUsize::new(0));
        let actually_extracted = Arc::new(AtomicUsize::new(0));
        let total = self.graph.inner.node_count();
        let total_completed = Arc::new(AtomicUsize::new(0));
        let node_modules = root.join("node_modules");
        std::fs::create_dir_all(&node_modules)?;
        let prefer_copy = self.prefer_copy
            || match self.cache.as_deref() {
                Some(cache) => supports_reflink(cache, &node_modules),
                None => false,
            };
        let validate = self.validate;
        stream
            .map(|idx| Ok((idx, concurrent_count.clone(), total_completed.clone(), actually_extracted.clone())))
            .try_for_each_concurrent(
                self.concurrency,
                move |(child_idx, concurrent_count, total_completed, actually_extracted)| async move {
                    if child_idx == self.graph.root {
                        return Ok(());
                    }

                    concurrent_count.fetch_add(1, atomic::Ordering::SeqCst);
                    let subdir = self
                        .graph
                        .node_path(child_idx)
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join("/node_modules/");
                    let target_dir = root.join("node_modules").join(&subdir);

                    let start = std::time::Instant::now();

                    if !target_dir.exists() {
                        self.graph[child_idx]
                            .package
                            .extract_to_dir(&target_dir, prefer_copy, validate)
                            .await?;
                        actually_extracted.fetch_add(1, atomic::Ordering::SeqCst);
                    }

                    if let Some(on_extract) = &self.on_extract_progress {
                        on_extract(&self.graph[child_idx].package);
                    }

                    tracing::trace!(
                        in_flight = concurrent_count.fetch_sub(1, atomic::Ordering::SeqCst) - 1,
                        "Extracted {} to {} in {:?}ms. {}/{total} done.",
                        self.graph[child_idx].package.name(),
                        target_dir.display(),
                        start.elapsed().as_millis(),
                        total_completed.fetch_add(1, atomic::Ordering::SeqCst) + 1,
                    );
                    Ok::<_, NodeMaintainerError>(())
                },
            )
            .await?;
        std::fs::write(
            node_modules.join(META_FILE_NAME),
            self.to_kdl()?.to_string(),
        )?;
        let actually_extracted = actually_extracted.load(atomic::Ordering::SeqCst);
        tracing::debug!(
            "Extracted {actually_extracted} package{} in {}ms.",
            if actually_extracted == 1 { "" } else { "s" },
            start.elapsed().as_millis(),
        );
        Ok(actually_extracted)
    }

    /// Links package binaries to their corresponding `node_modules/.bin`
    /// directories. On Windows, this will create `.cmd`, `.ps1`, and `sh`
    /// shims instead of link directly to the bins.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn link_bins(&self) -> Result<usize, NodeMaintainerError> {
        use walkdir::WalkDir;

        tracing::debug!("Linking bins...");
        let start = std::time::Instant::now();
        let root = &self.root;
        let linked = Arc::new(AtomicUsize::new(0));
        let bin_file_name = Some(OsStr::new(".bin"));
        let nm_file_name = Some(OsStr::new("node_modules"));
        for entry in WalkDir::new(root.join("node_modules"))
            .into_iter()
            .filter_entry(|e| {
                let path = e.path().file_name();
                path == bin_file_name || path == nm_file_name
            })
        {
            let entry = entry?;
            if entry.path().file_name() == bin_file_name {
                async_std::fs::remove_dir_all(entry.path()).await?;
            }
        }
        futures::stream::iter(self.graph.inner.node_indices())
            .map(|idx| Ok((idx, linked.clone())))
            .try_for_each_concurrent(self.concurrency, move |(idx, linked)| async move {
                if idx == self.graph.root {
                    return Ok(());
                }

                let subdir = self
                    .graph
                    .node_path(idx)
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join("/node_modules/");
                let package_dir = root.join("node_modules").join(subdir);
                let parent = package_dir.parent().expect("must have parent");
                let target_dir = if parent.file_name() == Some(OsStr::new("node_modules")) {
                    parent.join(".bin")
                } else {
                    // Scoped
                    parent.parent().expect("must have parent").join(".bin")
                };

                let build_mani = BuildManifest::from_path(package_dir.join("package.json"))
                    .map_err(|e| {
                        NodeMaintainerError::BuildManifestReadError(
                            package_dir.join("package.json"),
                            e,
                        )
                    })?;

                for (name, path) in &build_mani.bin {
                    let target_dir = target_dir.clone();
                    let to = target_dir.join(name);
                    let from = package_dir.join(path);
                    let name = name.clone();
                    async_std::task::spawn_blocking(move || {
                        // We only create a symlink if the target bin exists.
                        if from.symlink_metadata().is_ok() {
                            std::fs::create_dir_all(target_dir)?;
                            // TODO: use a DashMap here to prevent race conditions, maybe?
                            if let Ok(meta) = to.symlink_metadata() {
                                if meta.is_dir() {
                                    std::fs::remove_dir_all(&to)?;
                                } else {
                                    std::fs::remove_file(&to)?;
                                }
                            }
                            link_bin(&from, &to)?;
                            tracing::trace!(
                                "Linked bin for {} from {} to {}",
                                name,
                                from.display(),
                                to.display()
                            );
                        }
                        Ok::<_, NodeMaintainerError>(())
                    })
                    .await?;
                    linked.fetch_add(1, atomic::Ordering::SeqCst);
                }
                Ok::<_, NodeMaintainerError>(())
            })
            .await?;
        let linked = linked.load(atomic::Ordering::SeqCst);
        tracing::debug!(
            "Linked {linked} package bins in {}ms.",
            start.elapsed().as_millis()
        );
        Ok(linked)
    }

    /// Runs the `preinstall`, `install`, and `postinstall` lifecycle scripts,
    /// as well as linking the package bins as needed.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn rebuild(&self, ignore_scripts: bool) -> Result<(), NodeMaintainerError> {
        tracing::debug!("Running lifecycle scripts...");
        let start = std::time::Instant::now();
        if !ignore_scripts {
            self.run_scripts("preinstall").await?;
        }
        self.link_bins().await?;
        if !ignore_scripts {
            self.run_scripts("install").await?;
            self.run_scripts("postinstall").await?;
        }
        tracing::debug!(
            "Ran lifecycle scripts in {}ms.",
            start.elapsed().as_millis()
        );
        Ok(())
    }

    /// Concurrently executes the lifecycle scripts for the given event across
    /// all packages in the graph.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn run_scripts(&self, event: impl AsRef<str>) -> Result<(), NodeMaintainerError> {
        async fn inner(me: &NodeMaintainer, event: &str) -> Result<(), NodeMaintainerError> {
            tracing::debug!("Running {event} lifecycle scripts");
            let start = std::time::Instant::now();
            let root = &me.root;
            futures::stream::iter(me.graph.inner.node_indices())
                .map(Ok)
                .try_for_each_concurrent(me.script_concurrency, move |idx| async move {
                    if idx == me.graph.root {
                        return Ok::<_, NodeMaintainerError>(());
                    }

                    let subdir = me
                        .graph
                        .node_path(idx)
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join("/node_modules/");
                    let package_dir = root.join("node_modules").join(subdir);

                    let build_mani = BuildManifest::from_path(package_dir.join("package.json"))
                        .map_err(|e| {
                            NodeMaintainerError::BuildManifestReadError(
                                package_dir.join("package.json"),
                                e,
                            )
                        })?;

                    let name = me.graph[idx].package.name().to_string();
                    if build_mani.scripts.contains_key(event) {
                        let package_dir = package_dir.clone();
                        let root = root.clone();
                        let event = event.to_owned();
                        let span = tracing::info_span!("script::{name}::{event}");
                        let _span_enter = span.enter();
                        if let Some(on_script_start) = &me.on_script_start {
                            on_script_start(&me.graph[idx].package, &event);
                        }
                        std::mem::drop(_span_enter);
                        let mut script = async_std::task::spawn_blocking(move || {
                            OroScript::new(package_dir, event)?
                                .workspace_path(root)
                                .spawn()
                        })
                        .await?;
                        let stdout = script.stdout.take();
                        let stderr = script.stderr.take();
                        let stdout_name = name.clone();
                        let stderr_name = name.clone();
                        let stdout_on_line = me.on_script_line.clone();
                        let stderr_on_line = me.on_script_line.clone();
                        let stdout_span = span;
                        let stderr_span = stdout_span.clone();
                        futures::try_join!(
                            async_std::task::spawn_blocking(move || {
                                let _enter = stdout_span.enter();
                                if let Some(stdout) = stdout {
                                    for line in BufReader::new(stdout).lines() {
                                        let line = line?;
                                        tracing::debug!("stdout::{stdout_name}: {}", line);
                                        if let Some(on_script_line) = &stdout_on_line {
                                            on_script_line(&line);
                                        }
                                    }
                                }
                                Ok::<_, NodeMaintainerError>(())
                            }),
                            async_std::task::spawn_blocking(move || {
                                let _enter = stderr_span.enter();
                                if let Some(stderr) = stderr {
                                    for line in BufReader::new(stderr).lines() {
                                        let line = line?;
                                        tracing::debug!("stderr::{stderr_name}: {}", line);
                                        if let Some(on_script_line) = &stderr_on_line {
                                            on_script_line(&line);
                                        }
                                    }
                                }
                                Ok::<_, NodeMaintainerError>(())
                            }),
                            async_std::task::spawn_blocking(move || {
                                script.wait()?;
                                Ok::<_, NodeMaintainerError>(())
                            }),
                        )?;
                    }

                    Ok::<_, NodeMaintainerError>(())
                })
                .await?;
            tracing::debug!(
                "Ran lifecycle scripts for {event} in {}ms.",
                start.elapsed().as_millis()
            );
            Ok(())
        }
        inner(self, event.as_ref()).await
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn supports_reflink(src_dir: &Path, dest_dir: &Path) -> bool {
    let temp = match tempfile::NamedTempFile::new_in(src_dir) {
        Ok(t) => t,
        Err(e) => {
            tracing::debug!("error creating tempfile while checking for reflink support: {e}.");
            return false;
        }
    };
    match std::fs::write(&temp, "a") {
        Ok(_) => {}
        Err(e) => {
            tracing::debug!("error writing to tempfile while checking for reflink support: {e}.");
            return false;
        }
    };
    let tempdir = match tempfile::TempDir::new_in(dest_dir) {
        Ok(t) => t,
        Err(e) => {
            tracing::debug!(
                "error creating destination tempdir while checking for reflink support: {e}."
            );
            return false;
        }
    };
    let supports_reflink = reflink::reflink(temp.path(), tempdir.path().join("b"))
        .map(|_| true)
        .map_err(|e| {
            tracing::debug!(
                "reflink support check failed. Files will be hard linked or copied. ({e})"
            );
            e
        })
        .unwrap_or(false);

    if supports_reflink {
        tracing::debug!("Verified reflink support. Extracted data will use copy-on-write reflinks instead of hard links or full copies.")
    }

    supports_reflink
}

#[cfg(not(target_arch = "wasm32"))]
fn link_bin(from: &Path, to: &Path) -> Result<(), NodeMaintainerError> {
    #[cfg(windows)]
    oro_shim_bin::shim_bin(from, to)?;
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(from, to)?;
    }
    Ok(())
}
