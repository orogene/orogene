use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::ffi::OsStr;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{self, AtomicUsize};
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use async_std::fs;
use async_std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use colored::*;
#[cfg(not(target_arch = "wasm32"))]
use futures::TryStreamExt;
use futures::{StreamExt, TryFutureExt};
use nassun::client::{Nassun, NassunOpts};
use nassun::package::Package;
use nassun::PackageSpec;
use oro_common::{BuildManifest, CorgiManifest, CorgiVersionMetadata};
use oro_script::OroScript;
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use unicase::UniCase;
use url::Url;

use crate::edge::{DepType, Edge};
use crate::error::NodeMaintainerError;
use crate::{Graph, IntoKdl, Lockfile, LockfileNode, Node};

const DEFAULT_CONCURRENCY: usize = 50;
#[allow(dead_code)]
const META_FILE_NAME: &str = ".orogene-meta.kdl";

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
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn cache(mut self, cache: impl AsRef<Path>) -> Self {
        self.nassun_opts = self.nassun_opts.cache(PathBuf::from(cache.as_ref()));
        self.cache = Some(PathBuf::from(cache.as_ref()));
        self
    }

    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    pub fn kdl_lock(mut self, kdl_lock: impl IntoKdl) -> Result<Self, NodeMaintainerError> {
        let lock = Lockfile::from_kdl(kdl_lock)?;
        self.kdl_lock = Some(lock);
        Ok(self)
    }

    pub fn npm_lock(mut self, npm_lock: impl AsRef<str>) -> Result<Self, NodeMaintainerError> {
        let lock = Lockfile::from_npm(npm_lock)?;
        self.npm_lock = Some(lock);
        Ok(self)
    }

    pub fn registry(mut self, registry: Url) -> Self {
        self.nassun_opts = self.nassun_opts.registry(registry);
        self
    }

    pub fn scope_registry(mut self, scope: impl AsRef<str>, registry: Url) -> Self {
        self.nassun_opts = self.nassun_opts.scope_registry(scope, registry);
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn root(mut self, path: impl AsRef<Path>) -> Self {
        self.nassun_opts = self.nassun_opts.base_dir(path.as_ref());
        self.root = Some(PathBuf::from(path.as_ref()));
        self
    }

    pub fn default_tag(mut self, tag: impl AsRef<str>) -> Self {
        self.nassun_opts = self.nassun_opts.default_tag(tag);
        self
    }

    /// When extracting tarballs, prefer to copy files to their destination as
    /// separate, standalone files instead of hard linking them. Full copies
    /// will still happen when hard linking fails. Furthermore, on filesystems
    /// that support Copy-on-Write (zfs, btrfs, APFS (macOS), etc), this
    /// option will use that feature for all copies.
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

    pub async fn resolve_manifest(
        self,
        root: CorgiManifest,
    ) -> Result<NodeMaintainer, NodeMaintainerError> {
        let lockfile = self.get_lockfile().await?;
        let nassun = self.nassun_opts.build();
        let root_pkg = Nassun::dummy_from_manifest(root.clone());
        let mut nm = NodeMaintainer {
            nassun,
            graph: Default::default(),
            concurrency: DEFAULT_CONCURRENCY,
            cache: self.cache,
            prefer_copy: self.prefer_copy,
            validate: self.validate,
            root: self.root.unwrap_or_else(|| PathBuf::from(".")),
            actual_tree: None,
            on_resolution_added: self.on_resolution_added,
            on_resolve_progress: self.on_resolve_progress,
            on_prune_progress: self.on_prune_progress,
            on_extract_progress: self.on_extract_progress,
            on_script_start: self.on_script_start,
            on_script_line: self.on_script_line,
        };
        let node = nm.graph.inner.add_node(Node::new(root_pkg, root));
        nm.graph[node].root = node;
        nm.run_resolver(lockfile).await?;
        #[cfg(debug_assertions)]
        nm.graph.validate()?;
        Ok(nm)
    }

    pub async fn resolve_spec(
        self,
        root_spec: impl AsRef<str>,
    ) -> Result<NodeMaintainer, NodeMaintainerError> {
        let lockfile = self.get_lockfile().await?;
        let nassun = self.nassun_opts.build();
        let root_pkg = nassun.resolve(root_spec).await?;
        let mut nm = NodeMaintainer {
            nassun,
            graph: Default::default(),
            concurrency: DEFAULT_CONCURRENCY,
            cache: self.cache,
            prefer_copy: self.prefer_copy,
            validate: self.validate,
            root: self.root.unwrap_or_else(|| PathBuf::from(".")),
            actual_tree: None,
            on_resolution_added: self.on_resolution_added,
            on_resolve_progress: self.on_resolve_progress,
            on_prune_progress: self.on_prune_progress,
            on_extract_progress: self.on_extract_progress,
            on_script_start: self.on_script_start,
            on_script_line: self.on_script_line,
        };
        let corgi = root_pkg.corgi_metadata().await?.manifest;
        let node = nm.graph.inner.add_node(Node::new(root_pkg, corgi));
        nm.graph[node].root = node;
        nm.run_resolver(lockfile).await?;
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

#[derive(Debug, Clone)]
struct NodeDependency {
    name: UniCase<String>,
    spec: String,
    dep_type: DepType,
    node_idx: NodeIndex,
}

pub struct NodeMaintainer {
    nassun: Nassun,
    graph: Graph,
    concurrency: usize,
    #[allow(dead_code)]
    cache: Option<PathBuf>,
    #[allow(dead_code)]
    prefer_copy: bool,
    #[allow(dead_code)]
    validate: bool,
    #[allow(dead_code)]
    root: PathBuf,
    actual_tree: Option<Lockfile>,
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

impl NodeMaintainer {
    pub fn builder() -> NodeMaintainerOptions {
        NodeMaintainerOptions::new()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn resolve_manifest(
        root: CorgiManifest,
    ) -> Result<NodeMaintainer, NodeMaintainerError> {
        Self::builder().resolve_manifest(root).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn resolve_spec(
        root_spec: impl AsRef<str>,
    ) -> Result<NodeMaintainer, NodeMaintainerError> {
        Self::builder().resolve_spec(root_spec).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn render_to_file(&self, path: impl AsRef<Path>) -> Result<(), NodeMaintainerError> {
        fs::write(path.as_ref(), self.graph.render()).await?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn write_lockfile(&self, path: impl AsRef<Path>) -> Result<(), NodeMaintainerError> {
        fs::write(path.as_ref(), self.graph.to_kdl()?.to_string()).await?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn to_lockfile(&self) -> Result<crate::Lockfile, NodeMaintainerError> {
        self.graph.to_lockfile()
    }

    pub fn to_kdl(&self) -> Result<kdl::KdlDocument, NodeMaintainerError> {
        self.graph.to_kdl()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn render(&self) -> String {
        self.graph.render()
    }

    pub fn package_at_path(&self, path: &Path) -> Option<Package> {
        self.graph.package_at_path(path)
    }

    pub fn package_count(&self) -> usize {
        self.graph.inner.node_count()
    }

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

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn run_scripts(&self, event: impl AsRef<str>) -> Result<(), NodeMaintainerError> {
        async fn inner(me: &NodeMaintainer, event: &str) -> Result<(), NodeMaintainerError> {
            tracing::debug!("Running {event} lifecycle scripts");
            let start = std::time::Instant::now();
            let root = &me.root;
            futures::stream::iter(me.graph.inner.node_indices())
                .map(Ok)
                .try_for_each_concurrent(6, move |idx| async move {
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

    async fn run_resolver(
        &mut self,
        lockfile: Option<Lockfile>,
    ) -> Result<(), NodeMaintainerError> {
        #[cfg(not(target_arch = "wasm32"))]
        let start = std::time::Instant::now();

        #[cfg(not(target_arch = "wasm32"))]
        self.load_actual().await?;

        let (package_sink, package_stream) = futures::channel::mpsc::unbounded();
        let mut q = VecDeque::new();
        q.push_back(self.graph.root);

        // Number of dependencies queued for processing in `package_stream`
        let mut in_flight = 0;

        // Since we queue dependencies for multiple packages at once - it is
        // not unlikely that some of them would be duplicated by currently
        // fetched dependencies. Thus we maintain a mapping from "name@spec" to
        // a vector of `NodeDependency`s. When we will fetch the package - we
        // will apply it to all dependencies that need it.
        let fetches: BTreeMap<String, Vec<NodeDependency>> = BTreeMap::new();
        let fetches = Arc::new(Mutex::new(fetches));

        let mut package_stream = package_stream
            .map(|dep: NodeDependency| {
                let spec = format!("{}@{}", dep.name, dep.spec);
                let maybe_spec = if let Some(mut fetches) = fetches.try_lock() {
                    if let Some(list) = fetches.get_mut(&spec) {
                        // Package fetch is already in-flight, add dependency
                        // to the existing list.
                        list.push(dep);
                        None
                    } else {
                        // Fetch package since we are the first one to get here.
                        fetches.insert(spec.clone(), vec![dep]);
                        Some(spec)
                    }
                } else {
                    // Mutex is locked - fetch the package
                    Some(spec)
                };
                futures::future::ready(maybe_spec)
            })
            .filter_map(|maybe_spec| maybe_spec)
            .map(|spec| self.nassun.resolve(spec.clone()).map_ok(move |p| (p, spec)))
            .buffer_unordered(self.concurrency)
            .ready_chunks(self.concurrency);

        // Start iterating over the queue. We'll be adding things to it as we find them.
        while !q.is_empty() || in_flight != 0 {
            while let Some(node_idx) = q.pop_front() {
                let mut names = HashSet::new();
                let manifest = self.graph[node_idx].manifest.clone();
                // Grab all the deps from the current package and fire off a
                // lookup. These will be resolved concurrently.
                for ((name, spec), dep_type) in self.package_deps(node_idx, &manifest) {
                    // `dependencies` > `optionalDependencies` ->
                    // `peerDependencies` -> `devDependencies` (if we're looking
                    // at root)
                    let name = UniCase::new(name.clone());

                    if names.contains(&name) {
                        continue;
                    } else {
                        names.insert(name.clone());
                    }

                    let dep = NodeDependency {
                        name: name.clone(),
                        spec: spec.to_string(),
                        dep_type: dep_type.clone(),
                        node_idx,
                    };

                    if let Some(handler) = &self.on_resolution_added {
                        handler();
                    }

                    let requested = format!("{}@{}", dep.name, dep.spec).parse()?;

                    if let Some(_child_idx) = Self::satisfy_dependency(&mut self.graph, &dep)? {
                        if let Some(handler) = &self.on_resolve_progress {
                            handler(&self.graph[_child_idx].package);
                        }
                    }
                    // Walk up the current hierarchy to see if we find a
                    // dependency that already satisfies this request. If so,
                    // make a new edge and move on.
                    else {
                        // If we have a lockfile, first check if there's a
                        // dep there that would satisfy this.
                        let lock = if lockfile.is_some() {
                            &lockfile
                        } else {
                            // Fall back to the actual tree lock if it's there.
                            &self.actual_tree
                        };
                        if let Some(kdl_lock) = lock {
                            if let Some((package, lockfile_node)) = self
                                .satisfy_from_lockfile(
                                    &self.graph,
                                    node_idx,
                                    kdl_lock,
                                    &name,
                                    &requested,
                                )
                                .await?
                            {
                                let target_path = lockfile_node.path.clone();

                                let child_idx = Self::place_child(
                                    &mut self.graph,
                                    node_idx,
                                    package,
                                    &requested,
                                    dep_type,
                                    lockfile_node.into(),
                                    Some(target_path),
                                )
                                .await?;
                                q.push_back(child_idx);

                                if let Some(handler) = &self.on_resolve_progress {
                                    handler(&self.graph[child_idx].package);
                                }
                                continue;
                            }
                        }

                        // Otherwise, we have to fetch package metadata to
                        // create a new node (which we'll place later).
                        in_flight += 1;
                        package_sink.unbounded_send(dep)?;
                    };
                }
            }

            // Nothing in flight - don't await the stream
            if in_flight == 0 {
                continue;
            }

            // Order doesn't matter here: each node name is unique, so we
            // don't have to worry about races messing with placement.
            if let Some(packages) = package_stream.next().await {
                for res in packages {
                    let (package, spec) = res?;
                    let deps = fetches.lock().await.remove(&spec);

                    if let Some(deps) = deps {
                        in_flight -= deps.len();

                        let CorgiVersionMetadata {
                            manifest,
                            #[cfg(not(target_arch = "wasm32"))]
                            deprecated,
                            ..
                        } = &package.corgi_metadata().await?;

                        #[cfg(not(target_arch = "wasm32"))]
                        if let Some(deprecated) = deprecated {
                            tracing::warn!(
                                "{} {}@{}: {}",
                                "deprecated".magenta(),
                                manifest.name.as_ref().unwrap(),
                                manifest
                                    .version
                                    .as_ref()
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "unknown".into()),
                                deprecated
                            );
                        }

                        for dep in deps {
                            if let Some(_child_idx) =
                                Self::satisfy_dependency(&mut self.graph, &dep)?
                            {
                                if let Some(handler) = &self.on_resolve_progress {
                                    handler(&self.graph[_child_idx].package);
                                }
                                continue;
                            }

                            let requested = format!("{}@{}", dep.name, dep.spec).parse()?;

                            let child_idx = Self::place_child(
                                &mut self.graph,
                                dep.node_idx,
                                package.clone(),
                                &requested,
                                dep.dep_type,
                                manifest.clone(),
                                None,
                            )
                            .await?;

                            q.push_back(child_idx);

                            if let Some(handler) = &self.on_resolve_progress {
                                handler(&self.graph[child_idx].package);
                            }
                        }
                    }
                }

                // We sort the current queue so we consider more shallow
                // dependencies first, and we also sort alphabetically.
                q.make_contiguous().sort_by(|a_idx, b_idx| {
                    let a = &self.graph[*a_idx];
                    let b = &self.graph[*b_idx];
                    match a.depth(&self.graph).cmp(&b.depth(&self.graph)) {
                        Ordering::Equal => a.package.name().cmp(b.package.name()),
                        other => other,
                    }
                })
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        tracing::debug!(
            "Resolved graph of {} nodes in {}ms",
            self.graph.inner.node_count(),
            start.elapsed().as_millis()
        );
        Ok(())
    }

    fn satisfy_dependency(
        graph: &mut Graph,
        dep: &NodeDependency,
    ) -> Result<Option<NodeIndex>, NodeMaintainerError> {
        if let Some(satisfier_idx) = graph.find_by_name(dep.node_idx, &dep.name)? {
            let requested = format!("{}@{}", dep.name, dep.spec).parse()?;
            if graph[satisfier_idx]
                .package
                .resolved()
                .satisfies(&requested)?
            {
                let edge_idx = graph.inner.add_edge(
                    dep.node_idx,
                    satisfier_idx,
                    Edge::new(requested, dep.dep_type.clone()),
                );
                graph[dep.node_idx]
                    .dependencies
                    .insert(dep.name.clone(), edge_idx);
                return Ok(Some(satisfier_idx));
            }
            return Ok(None);
        }
        Ok(None)
    }

    async fn satisfy_from_lockfile(
        &self,
        graph: &Graph,
        dependent_idx: NodeIndex,
        lockfile: &Lockfile,
        name: &UniCase<String>,
        requested: &PackageSpec,
    ) -> Result<Option<(Package, LockfileNode)>, NodeMaintainerError> {
        let mut path = graph.node_path(dependent_idx);
        let mut last_loop = false;
        loop {
            if path.is_empty() {
                last_loop = true;
            }
            path.push_back(name.clone());
            let path_str = UniCase::from(
                path.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join("/node_modules/"),
            );
            path.pop_back();
            if let Some(lockfile_node) = lockfile.packages().get(&path_str) {
                if let Some(package) = lockfile_node.to_package(&self.nassun).await? {
                    if package.resolved().satisfies(requested)? {
                        return Ok(Some((package, lockfile_node.clone())));
                    } else {
                        // TODO: Log this We found a lockfile node in a place
                        // where it would be loaded, but it doesn't satisfy the
                        // actual request, so it would be wrong. Return None here
                        // so the node gets re-resolved.
                        return Ok(None);
                    }
                }
            }
            if last_loop {
                break;
            }
            path.pop_back();
        }
        Ok(None)
    }

    async fn place_child(
        graph: &mut Graph,
        dependent_idx: NodeIndex,
        package: Package,
        requested: &PackageSpec,
        dep_type: DepType,
        corgi: CorgiManifest,
        target_path: Option<Vec<UniCase<String>>>,
    ) -> Result<NodeIndex, NodeMaintainerError> {
        let child_name = UniCase::new(package.name().to_string());
        let child_node = Node::new(package, corgi);
        let child_idx = graph.inner.add_node(child_node);
        graph[child_idx].root = graph.root;
        // We needed to generate the node index before setting it in the node,
        // so we do that now.
        graph[child_idx].idx = child_idx;

        // Edges represent the logical dependency relationship (not the
        // hierarchy location).
        let edge_idx = graph.inner.add_edge(
            dependent_idx,
            child_idx,
            Edge::new(requested.clone(), dep_type),
        );

        let mut target_idx = graph.root;
        let mut found_in_path = true;
        // If we got a suggested target path, we'll try to use that first.
        if let Some(target_path) = target_path {
            for segment in target_path.iter().take(target_path.len() - 1) {
                if let Some(new_target) = &graph[target_idx].children.get(segment) {
                    target_idx = **new_target;
                } else {
                    // We couldn't find the target path. We'll just place the
                    // node in the highest possible location.
                    found_in_path = false;
                    break;
                }
            }
        } else {
            found_in_path = false;
        }

        if !found_in_path {
            // If we didn't have a path, or the path wasn't correct, we
            // calculate the highest hierarchy location that we can place this
            // node in.
            let mut parent_idx = Some(dependent_idx);
            target_idx = dependent_idx;
            'outer: while let Some(curr_target_idx) = parent_idx {
                if let Some(resolved) = graph.resolve_dep(curr_target_idx, &child_name) {
                    for edge_ref in graph.inner.edges_directed(resolved, Direction::Incoming) {
                        let (from, _) = graph
                            .inner
                            .edge_endpoints(edge_ref.id())
                            .expect("Where did the edge go?!?!");
                        if graph.is_ancestor(curr_target_idx, from)
                            && !graph[resolved].package.resolved().satisfies(requested)?
                        {
                            break 'outer;
                        }
                    }
                }

                // No conflict yet. Let's try to go higher!
                target_idx = curr_target_idx;
                parent_idx = graph[curr_target_idx].parent;
            }
        }
        {
            // Now we set backlinks: first, the dependent node needs to point
            // to the child, wherever it is in the graph.
            let dependent = &mut graph[dependent_idx];
            dependent.dependencies.insert(child_name.clone(), edge_idx);
        }
        // Finally, we put everything in its place.
        {
            let mut child_node = &mut graph[child_idx];
            // The parent is the _hierarchy_ location, so we set its parent
            // accordingly.
            child_node.parent = Some(target_idx);
        }
        {
            // Finally, we add the backlink from the parent node to the child.
            let node = &mut graph[target_idx];
            node.children.insert(child_name, child_idx);
        }
        Ok(child_idx)
    }

    fn package_deps<'a, 'b>(
        &'a self,
        node_idx: NodeIndex,
        manifest: &'b CorgiManifest,
    ) -> Box<dyn Iterator<Item = ((&'b String, &'b String), DepType)> + 'b + Send> {
        let deps = manifest
            .dependencies
            .iter()
            .map(|x| (x, DepType::Prod))
            .chain(
                manifest
                    .optional_dependencies
                    .iter()
                    .map(|x| (x, DepType::Opt)),
                // TODO: Place these properly.
                // )
                // .chain(
                //     manifest
                //         .peer_dependencies
                //         .iter()
                //         .map(|x| (x, DepType::Peer)),
            );

        if node_idx == self.graph.root {
            Box::new(deps.chain(manifest.dev_dependencies.iter().map(|x| (x, DepType::Dev))))
        } else {
            Box::new(deps)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn load_actual(&mut self) -> Result<(), NodeMaintainerError> {
        let meta = self.root.join("node_modules").join(META_FILE_NAME);
        self.actual_tree = async_std::fs::read_to_string(&meta)
            .await
            .ok()
            .and_then(|lock| Lockfile::from_kdl(lock).ok());
        if self.actual_tree.is_none() && meta.exists() {
            // If anything went wrong, we go ahead and delete the meta file,
            // if it exists, because it's probably corrupted.
            async_std::fs::remove_file(meta).await?;
        }
        Ok(())
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

fn link_bin(from: &Path, to: &Path) -> Result<(), NodeMaintainerError> {
    #[cfg(windows)]
    oro_shim_bin::shim_bin(from, to)?;
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(from, to)?;
    }
    Ok(())
}
