use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{self, AtomicUsize};

#[cfg(not(target_arch = "wasm32"))]
use async_std::fs;
use async_std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use colored::*;
#[cfg(not(target_arch = "wasm32"))]
use futures::TryStreamExt;
use futures::{StreamExt, TryFutureExt};
#[cfg(not(target_arch = "wasm32"))]
use indicatif::{ProgressBar, ProgressStyle};
use nassun::client::{Nassun, NassunOpts};
use nassun::package::Package;
use nassun::PackageSpec;
use oro_common::{CorgiManifest, CorgiVersionMetadata};
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use unicase::UniCase;
use url::Url;

use crate::edge::{DepType, Edge};
use crate::error::NodeMaintainerError;
use crate::{Graph, IntoKdl, Lockfile, LockfileNode, Node};

const DEFAULT_CONCURRENCY: usize = 50;
const META_FILE_NAME: &str = ".orogene-meta.kdl";

#[derive(Debug, Clone)]
pub struct NodeMaintainerOptions {
    nassun_opts: NassunOpts,
    concurrency: usize,
    kdl_lock: Option<Lockfile>,
    npm_lock: Option<Lockfile>,

    #[cfg(not(target_arch = "wasm32"))]
    progress_bar: bool,
    #[allow(dead_code)]
    cache: Option<PathBuf>,
    #[allow(dead_code)]
    prefer_copy: bool,
    #[allow(dead_code)]
    validate: bool,
    #[allow(dead_code)]
    root: Option<PathBuf>,
}

impl NodeMaintainerOptions {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn progress_bar(mut self, progress_bar: bool) -> Self {
        self.progress_bar = progress_bar;
        self
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

    pub async fn resolve_manifest(
        self,
        root: CorgiManifest,
    ) -> Result<NodeMaintainer, NodeMaintainerError> {
        let nassun = self.nassun_opts.build();
        let root_pkg = Nassun::dummy_from_manifest(root.clone());
        let mut nm = NodeMaintainer {
            nassun,
            graph: Default::default(),
            concurrency: DEFAULT_CONCURRENCY,
            #[cfg(not(target_arch = "wasm32"))]
            progress_bar: self.progress_bar,
            cache: self.cache,
            prefer_copy: self.prefer_copy,
            validate: self.validate,
            root: self.root.unwrap_or_else(|| PathBuf::from(".")),
            actual_tree: None,
        };
        let node = nm.graph.inner.add_node(Node::new(root_pkg, root));
        nm.graph[node].root = node;
        nm.run_resolver(self.kdl_lock.or(self.npm_lock)).await?;
        #[cfg(debug_assertions)]
        nm.graph.validate()?;
        Ok(nm)
    }

    pub async fn resolve_spec(
        self,
        root_spec: impl AsRef<str>,
    ) -> Result<NodeMaintainer, NodeMaintainerError> {
        let nassun = self.nassun_opts.build();
        let root_pkg = nassun.resolve(root_spec).await?;
        let mut nm = NodeMaintainer {
            nassun,
            graph: Default::default(),
            concurrency: DEFAULT_CONCURRENCY,
            #[cfg(not(target_arch = "wasm32"))]
            progress_bar: self.progress_bar,
            cache: self.cache,
            prefer_copy: self.prefer_copy,
            validate: self.validate,
            root: self.root.unwrap_or_else(|| PathBuf::from(".")),
            actual_tree: None,
        };
        let corgi = root_pkg.corgi_metadata().await?.manifest;
        let node = nm.graph.inner.add_node(Node::new(root_pkg, corgi));
        nm.graph[node].root = node;
        nm.run_resolver(self.kdl_lock.or(self.npm_lock)).await?;
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
            #[cfg(not(target_arch = "wasm32"))]
            progress_bar: false,
            cache: None,
            prefer_copy: false,
            validate: false,
            root: None,
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
    #[cfg(not(target_arch = "wasm32"))]
    progress_bar: bool,
    cache: Option<PathBuf>,
    prefer_copy: bool,
    validate: bool,
    root: PathBuf,
    actual_tree: Option<Lockfile>,
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

    #[cfg(not(target_arch = "wasm32"))]
    async fn prune(&self) -> Result<(), NodeMaintainerError> {
        use walkdir::WalkDir;

        let prefix = self.root.join("node_modules");

        if !prefix.exists() {
            return Ok(());
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

            tracing::info!("No metadata file found in node_modules/. Pruned entire node_modules/ directory in {}ms.", start.elapsed().as_micros() / 1000);

            return Ok(());
        }

        let nm_osstr = Some(std::ffi::OsStr::new("node_modules"));
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

                if entry_path.file_name() == nm_osstr {
                    // We don't want to skip node_modules themselves
                    return true;
                }

                if entry_path
                    .file_name()
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
                || file_name
                    .map(|s| s.to_string_lossy().starts_with('@'))
                    .unwrap_or(false)
            {
                continue;
            } else if entry.file_type().is_dir() {
                async_std::fs::remove_dir_all(entry.path()).await?;
            } else {
                async_std::fs::remove_file(entry.path()).await?;
            }
        }

        if extraneous_packages == 0 {
            tracing::info!(
                "Nothing to prune. Completed check in {}ms.",
                start.elapsed().as_micros() / 1000
            );
        } else {
            tracing::info!(
                "Pruned {extraneous_packages} extraneous package{} in {}ms.",
                start.elapsed().as_micros() / 1000,
                if extraneous_packages == 1 { "" } else { "s" },
            );
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn extract(&self) -> Result<(), NodeMaintainerError> {
        let pb = if self.progress_bar {
            ProgressBar::new(self.graph.inner.node_count() as u64 - 1).with_style(
                ProgressStyle::default_bar()
                    .template("💾 {bar:40} [{pos}/{len}] {wide_msg:.dim}")
                    .unwrap(),
            )
        } else {
            ProgressBar::hidden()
        };

        let start = std::time::Instant::now();

        self.prune().await?;

        let root = &self.root;
        let stream = futures::stream::iter(self.graph.inner.node_indices());
        let concurrent_count = Arc::new(AtomicUsize::new(0));
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
        let pbref = &pb;
        stream
            .map(|idx| Ok((idx, concurrent_count.clone(), total_completed.clone())))
            .try_for_each_concurrent(
                self.concurrency,
                move |(child_idx, concurrent_count, total_completed)| async move {
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
                    }

                    pbref.inc(1);
                    pbref.set_message(format!("{:?}", self.graph[child_idx].package.resolved()));

                    tracing::debug!(
                        "Extracted {} to {} in {:?}ms. {}/{total} done. {} in flight.",
                        self.graph[child_idx].package.name(),
                        target_dir.display(),
                        start.elapsed().as_millis(),
                        total_completed.fetch_add(1, atomic::Ordering::SeqCst) + 1,
                        concurrent_count.fetch_sub(1, atomic::Ordering::SeqCst) - 1
                    );
                    Ok::<_, NodeMaintainerError>(())
                },
            )
            .await?;
        std::fs::write(
            node_modules.join(META_FILE_NAME),
            self.to_kdl()?.to_string(),
        )?;
        tracing::info!(
            "Extracted {total} package{} in {}ms.",
            if total == 1 { "" } else { "s" },
            start.elapsed().as_millis(),
        );
        if self.progress_bar {
            pb.finish_and_clear();
            println!("💾 Linked!");
        }
        Ok(())
    }

    async fn run_resolver(
        &mut self,
        lockfile: Option<Lockfile>,
    ) -> Result<(), NodeMaintainerError> {
        #[cfg(not(target_arch = "wasm32"))]
        let start = std::time::Instant::now();
        #[cfg(not(target_arch = "wasm32"))]
        let pb = if self.progress_bar {
            ProgressBar::new(0).with_style(
                ProgressStyle::default_bar()
                    .template("🔍 {bar:40} [{pos}/{len}] {wide_msg:.dim}")
                    .unwrap(),
            )
        } else {
            ProgressBar::hidden()
        };

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

                    #[cfg(not(target_arch = "wasm32"))]
                    pb.inc_length(1);

                    let requested = format!("{}@{}", dep.name, dep.spec).parse()?;

                    if let Some(_child_idx) = Self::satisfy_dependency(&mut self.graph, &dep)? {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            pb.inc(1);
                            pb.set_message(format!(
                                "{:?}",
                                self.graph[_child_idx].package.resolved()
                            ));
                        };
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

                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    pb.inc(1);
                                    pb.set_message(format!(
                                        "{:?}",
                                        self.graph[child_idx].package.resolved()
                                    ));
                                };
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
                            pb.suspend(|| {
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
                            });
                        }

                        for dep in deps {
                            if let Some(_child_idx) =
                                Self::satisfy_dependency(&mut self.graph, &dep)?
                            {
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    pb.inc(1);
                                    pb.set_message(format!(
                                        "{:?}",
                                        self.graph[_child_idx].package.resolved()
                                    ));
                                };
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

                            #[cfg(not(target_arch = "wasm32"))]
                            if self.progress_bar {
                                pb.inc(1);
                                pb.set_message(format!(
                                    "{:?}",
                                    self.graph[child_idx].package.resolved()
                                ));
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
        {
            pb.finish_and_clear();
            println!("🔍 Resolved!");
        };

        #[cfg(not(target_arch = "wasm32"))]
        tracing::info!(
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
            tracing::info!("error creating tempfile while checking for reflink support: {e}.");
            return false;
        }
    };
    match std::fs::write(&temp, "a") {
        Ok(_) => {}
        Err(e) => {
            tracing::info!("error writing to tempfile while checking for reflink support: {e}.");
            return false;
        }
    };
    let tempdir = match tempfile::TempDir::new_in(dest_dir) {
        Ok(t) => t,
        Err(e) => {
            tracing::info!(
                "error creating destination tempdir while checking for reflink support: {e}."
            );
            return false;
        }
    };
    let supports_reflink = reflink::reflink(temp.path(), tempdir.path().join("b"))
        .map(|_| true)
        .map_err(|e| {
            tracing::info!(
                "reflink support check failed. Files will be hard linked or copied. ({e})"
            );
            e
        })
        .unwrap_or(false);

    if supports_reflink {
        tracing::info!("Verified reflink support. Extracted data will use copy-on-write reflinks instead of hard links or full copies.")
    }

    supports_reflink
}
