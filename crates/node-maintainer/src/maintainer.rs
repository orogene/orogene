use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::{FutureExt, StreamExt, TryFutureExt, TryStreamExt};
use nassun::{Nassun, NassunOpts, Package, PackageSpec};
use oro_common::{CorgiManifest, CorgiVersionMetadata};
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use tokio::sync::Mutex;
use unicase::UniCase;
use url::Url;

use crate::edge::{DepType, Edge};
use crate::error::NodeMaintainerError;
use crate::{Graph, IntoKdl, Lockfile, LockfileNode, Node};

const DEFAULT_PARALLELISM: usize = 50;

#[derive(Debug, Clone)]
pub struct NodeMaintainerOptions {
    nassun_opts: NassunOpts,
    parallelism: usize,
    kdl_lock: Option<Lockfile>,
    npm_lock: Option<Lockfile>,
}

impl NodeMaintainerOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cache(mut self, cache: impl AsRef<Path>) -> Self {
        self.nassun_opts = self.nassun_opts.cache(PathBuf::from(cache.as_ref()));
        self
    }

    pub fn parallelism(mut self, parallelism: usize) -> Self {
        self.parallelism = parallelism;
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

    pub fn base_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.nassun_opts = self.nassun_opts.base_dir(path);
        self
    }

    pub fn default_tag(mut self, tag: impl AsRef<str>) -> Self {
        self.nassun_opts = self.nassun_opts.default_tag(tag);
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
            parallelism: DEFAULT_PARALLELISM,
        };
        let node = nm.graph.inner.add_node(Node::new(root_pkg, root.into()));
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
            parallelism: DEFAULT_PARALLELISM,
        };
        let corgi = root_pkg.corgi_metadata().await?;
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
            parallelism: DEFAULT_PARALLELISM,
            kdl_lock: None,
            npm_lock: None,
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
    parallelism: usize,
}

impl NodeMaintainer {
    pub fn builder() -> NodeMaintainerOptions {
        NodeMaintainerOptions::new()
    }

    pub async fn resolve_manifest(
        root: CorgiManifest,
    ) -> Result<NodeMaintainer, NodeMaintainerError> {
        Self::builder().resolve_manifest(root).await
    }

    pub async fn resolve_spec(
        root_spec: impl AsRef<str>,
    ) -> Result<NodeMaintainer, NodeMaintainerError> {
        Self::builder().resolve_spec(root_spec).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn render_to_file(&self, path: impl AsRef<Path>) -> Result<(), NodeMaintainerError> {
        std::fs::write(path.as_ref(), self.graph.render())?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn write_lockfile(&self, path: impl AsRef<Path>) -> Result<(), NodeMaintainerError> {
        std::fs::write(path.as_ref(), self.graph.to_kdl()?.to_string())?;
        Ok(())
    }

    pub fn to_lockfile(&self) -> Result<crate::Lockfile, NodeMaintainerError> {
        self.graph.to_lockfile()
    }

    pub fn to_kdl(&self) -> Result<kdl::KdlDocument, NodeMaintainerError> {
        self.graph.to_kdl()
    }

    pub fn render(&self) -> String {
        self.graph.render()
    }

    pub fn package_at_path(&self, path: &Path) -> Result<Option<Package>, NodeMaintainerError> {
        self.graph.package_at_path(path)
    }

    pub async fn extract_to(&self, path: impl AsRef<Path>) -> Result<(), NodeMaintainerError> {
        async fn inner(me: &NodeMaintainer, path: &Path) -> Result<(), NodeMaintainerError> {
            let mut indices = me
                .graph
                .inner
                .node_indices()
                .filter(|idx| *idx != me.graph.root)
                .collect::<Vec<_>>();
            // Let's pack the stream with out slowest downloads first, so they
            // don't start late and push the overall time window forward.
            indices.sort_unstable_by_key(|idx| me.graph[*idx].tarball_size);

            let (package_sink, package_stream) = futures::channel::mpsc::channel(me.parallelism);

            let sink_clone = package_sink.clone();
            let mut sink_clone2 = package_sink.clone();
            let mut sink_clone3 = package_sink.clone();
            let stream = futures::stream::iter(indices.iter().rev());
            let tarball_downloads = stream
                .map(|idx| Ok((idx, sink_clone.clone())))
                .try_for_each_concurrent(me.parallelism, |(idx, mut package_sink)| async move {
                    let child_idx = *idx;
                    tracing::debug!(
                        "Downloading {} ({:?} bytes, {:?} files)",
                        me.graph[child_idx].package.name(),
                        me.graph[child_idx].tarball_size,
                        me.graph[child_idx].tarball_file_count
                    );
                    let target_dir = path.join("node_modules").join(
                        me.graph
                            .node_path(child_idx)
                            .iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                            .join("/node_modules/"),
                    );

                    let start = std::time::Instant::now();
                    let tarball = me.graph[child_idx]
                        .package
                        .tarball()
                        .await?
                        .to_temp()
                        .await?;
                    tracing::debug!(
                        "Downloaded {} in {:?}ms.",
                        me.graph[child_idx].package.name(),
                        start.elapsed().as_millis(),
                    );
                    package_sink.try_send((
                        me.graph[child_idx].package.name(),
                        tarball,
                        target_dir,
                    ))?;
                    Ok::<_, NodeMaintainerError>(())
                })
                .and_then(|_| async move {
                    sink_clone2.close_channel();
                    Ok(())
                })
                .map_err(move |e| {
                    sink_clone3.close_channel();
                    e
                });
            let tarball_stream = package_stream.map(Ok).try_for_each_concurrent(
                me.parallelism * 5,
                |(name, tarball, target_dir)| async move {
                    let start = std::time::Instant::now();
                    tracing::debug!("Extracting {} to {:?}", name, target_dir);
                    tokio::task::spawn_blocking(|| tarball.extract_to_dir(target_dir)).await??;
                    tracing::debug!("Extracted {} in {:?}ms", name, start.elapsed().as_millis());
                    Ok::<_, NodeMaintainerError>(())
                },
            );
            futures::future::try_join(tarball_downloads.boxed(), tarball_stream.boxed()).await?;
            Ok(())
        }
        inner(self, path.as_ref()).await
    }

    async fn run_resolver(
        &mut self,
        lockfile: Option<Lockfile>,
    ) -> Result<(), NodeMaintainerError> {
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
        let fetches: HashMap<String, Vec<NodeDependency>> = HashMap::new();
        let fetches = Arc::new(Mutex::new(fetches));

        let mut package_stream = package_stream
            .filter_map(|dep: NodeDependency| async {
                let spec = format!("{}@{}", dep.name, dep.spec);
                let mut fetches = fetches.lock().await;
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
            })
            .map(|spec| self.nassun.resolve(spec.clone()).map_ok(move |p| (p, spec)))
            .buffer_unordered(self.parallelism)
            .ready_chunks(self.parallelism)
            .boxed();

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

                    let requested = format!("{}@{}", dep.name, dep.spec).parse()?;

                    // Walk up the current hierarchy to see if we find a
                    // dependency that already satisfies this request. If so,
                    // make a new edge and move on.
                    if !Self::satisfy_dependency(&mut self.graph, &dep)? {
                        // If we have a lockfile, first check if there's a
                        // dep there that would satisfy this.
                        if let Some(kdl_lock) = &lockfile {
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
                                q.push_back(
                                    Self::place_child(
                                        &mut self.graph,
                                        node_idx,
                                        package,
                                        &requested,
                                        dep_type,
                                        lockfile_node.into(),
                                        Some(target_path),
                                    )
                                    .await?,
                                );
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
                        let metadata = package.corgi_metadata().await?;

                        for dep in deps {
                            if Self::satisfy_dependency(&mut self.graph, &dep)? {
                                continue;
                            }

                            let requested = format!("{}@{}", dep.name, dep.spec).parse()?;
                            q.push_back(
                                Self::place_child(
                                    &mut self.graph,
                                    dep.node_idx,
                                    package.clone(),
                                    &requested,
                                    dep.dep_type,
                                    metadata.clone(),
                                    None,
                                )
                                .await?,
                            );
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

        Ok(())
    }

    fn satisfy_dependency(
        graph: &mut Graph,
        dep: &NodeDependency,
    ) -> Result<bool, NodeMaintainerError> {
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
                return Ok(true);
            }
            return Ok(false);
        }
        Ok(false)
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
        corgi: CorgiVersionMetadata,
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
}
