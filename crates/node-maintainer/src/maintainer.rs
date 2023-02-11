use std::cmp::Ordering;
use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::sync::atomic;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use async_std::fs;
use futures::channel::mpsc;
use futures::lock::Mutex;
use futures::{StreamExt, TryStreamExt};
use kdl::KdlDocument;
use nassun::{Nassun, NassunOpts, Package};
use oro_common::CorgiManifest;
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use unicase::UniCase;
use url::Url;

use crate::edge::{DepType, Edge};
use crate::error::NodeMaintainerError;
use crate::{Graph, IntoKdl, Lockfile, Node};

#[derive(Debug, Clone, Default)]
pub struct NodeMaintainerOptions {
    nassun_opts: NassunOpts,
}

impl NodeMaintainerOptions {
    pub fn new() -> Self {
        Self::default()
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

    pub async fn from_kdl(self, kdl: impl IntoKdl) -> Result<NodeMaintainer, NodeMaintainerError> {
        async fn inner(
            me: NodeMaintainerOptions,
            kdl: KdlDocument,
        ) -> Result<NodeMaintainer, NodeMaintainerError> {
            let nassun = me.nassun_opts.build();
            let graph = Lockfile::from_kdl(kdl)?.into_graph(&nassun).await?;
            let mut nm = NodeMaintainer { nassun, graph };
            nm.run_resolver().await?;
            Ok(nm)
        }
        inner(self, kdl.into_kdl()?).await
    }

    pub async fn resolve(
        self,
        root_spec: impl AsRef<str>,
    ) -> Result<NodeMaintainer, NodeMaintainerError> {
        let nassun = self.nassun_opts.build();
        let package = nassun.resolve(root_spec).await?;
        let mut nm = NodeMaintainer {
            nassun,
            graph: Default::default(),
        };
        let node = nm.graph.inner.add_node(Node::new(package));
        nm.graph[node].root = node;
        nm.run_resolver().await?;
        Ok(nm)
    }
}

pub struct NodeDependency {
    name: UniCase<String>,
    spec: String,
    dep_type: DepType,
    node_idx: NodeIndex,
}

pub struct NodeMaintainer {
    nassun: Nassun,
    graph: Graph,
}

impl NodeMaintainer {
    pub fn builder() -> NodeMaintainerOptions {
        NodeMaintainerOptions::new()
    }

    pub async fn resolve(
        root_spec: impl AsRef<str>,
    ) -> Result<NodeMaintainer, NodeMaintainerError> {
        Self::builder().resolve(root_spec).await
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

    pub fn to_resolved_tree(&self) -> Result<crate::Lockfile, NodeMaintainerError> {
        self.graph.to_resolved_tree()
    }

    pub fn to_kdl(&self) -> Result<kdl::KdlDocument, NodeMaintainerError> {
        self.graph.to_kdl()
    }

    pub fn render(&self) -> String {
        self.graph.render()
    }

    async fn run_resolver(&mut self) -> Result<(), NodeMaintainerError> {
        let idx_queue = Arc::new(Mutex::new(VecDeque::from([self.graph.root])));

        // Number of nodes in processing. When this drops to zero we close
        // both idx_receiver and dep_receiver so that streams end.
        let in_flight_count = Arc::new(atomic::AtomicUsize::new(1));

        let nassun = Arc::new(&self.nassun);
        let graph = Arc::new(Mutex::new(&mut self.graph));

        let (idx_sender, idx_receiver) = mpsc::unbounded::<()>();
        idx_sender.unbounded_send(())?;

        let (dep_sender, dep_receiver) = mpsc::unbounded::<NodeDependency>();

        let dep_receiver = dep_receiver.map(|node_dep| {
            let idx_queue = idx_queue.clone();
            let in_flight_count = in_flight_count.clone();
            let graph = graph.clone();
            let nassun = nassun.clone();
            let idx_sender = idx_sender.clone();
            let dep_sender = dep_sender.clone();

            async move {
                let package = nassun.resolve(&node_dep.name).await?;

                let mut graph = graph.lock().await;
                let satisfies = Self::satisfy_dependency(&mut graph, &node_dep)?;
                if satisfies {
                    // Terminate early because another package has satisfied
                    // the dependency.
                    if in_flight_count.fetch_sub(1, atomic::Ordering::SeqCst) == 1 {
                        idx_sender.close_channel();
                        dep_sender.close_channel();
                    }
                    return Ok::<_, NodeMaintainerError>(());
                }
                let child_idx =
                    Self::place_child(&mut graph, node_dep.node_idx, package, node_dep.dep_type)?;

                // Important: this has to be locked *after* locking the graph
                // to avoid deadlocks between two dep_receiver and idx_receiver.
                let mut idx_queue = idx_queue.lock().await;
                idx_queue.push_back(child_idx);

                // We sort the current queue so we consider more shallow
                // dependencies first, and we also sort alphabetically.
                idx_queue.make_contiguous().sort_by(|a_idx, b_idx| {
                    let a = &graph[*a_idx];
                    let b = &graph[*b_idx];
                    match a.depth(&graph).cmp(&b.depth(&graph)) {
                        Ordering::Equal => a.package.name().cmp(b.package.name()),
                        other => other,
                    }
                });

                // Drop mutexes early so that idx_receiver can run without
                // blocking.
                drop(idx_queue);
                drop(graph);

                // Notify idx_receiver about new queue element.
                idx_sender.unbounded_send(())?;

                Ok::<_, NodeMaintainerError>(())
            }
        });

        let idx_receiver =
            idx_receiver.map(|()| {
                let graph = graph.clone();
                let idx_queue = idx_queue.clone();
                let in_flight_count = in_flight_count.clone();
                let idx_sender = idx_sender.clone();
                let dep_sender = dep_sender.clone();

                async move {
                    // Same as above: lock on graph first, and idx_queue later
                    let mut graph = graph.lock().await;
                    let node_idx = idx_queue.lock().await.pop_front().ok_or(
                        NodeMaintainerError::MiscError("Unexpected end of idx_queue".to_string()),
                    )?;

                    let mut names = HashSet::new();
                    let manifest = graph[node_idx].package.corgi_metadata().await?.manifest;

                    let mut deps = Vec::new();

                    // Grab all the deps from the current package and fire off a
                    // lookup. These will be resolved concurrently.
                    for ((name, spec), dep_type) in Self::package_deps(&graph, node_idx, &manifest)
                    {
                        // `dependencies` > `optionalDependencies` ->
                        // `peerDependencies` -> `devDependencies` (if we're looking
                        // at root)
                        let name = UniCase::new(name.clone());
                        if names.contains(&name) {
                            continue;
                        }

                        let node_dep = NodeDependency {
                            name: name.clone(),
                            spec: spec.to_string(),
                            dep_type: dep_type.clone(),
                            node_idx,
                        };

                        // Walk up the current hierarchy to see if we find a
                        // dependency that already satisfies this request. If so,
                        // make a new edge and move on.
                        let satisfies = Self::satisfy_dependency(&mut graph, &node_dep)?;
                        if satisfies {
                            names.insert(name);
                            continue;
                        }

                        // Otherwise, we have to fetch package metadata to
                        // create a new node (which we'll place later).
                        in_flight_count.fetch_add(1, atomic::Ordering::SeqCst);
                        deps.push(node_dep);
                        names.insert(name);
                    }

                    // Release mutex so that code below doesn't dead lock
                    drop(graph);

                    for node_dep in deps {
                        dep_sender.unbounded_send(node_dep)?;
                    }

                    if in_flight_count.fetch_sub(1, atomic::Ordering::SeqCst) == 1 {
                        idx_sender.close_channel();
                        dep_sender.close_channel();
                    }

                    Ok::<(), NodeMaintainerError>(())
                }
            });

        futures::stream::select(
            // Fetch dependencies at high concurrency and unordered because
            // we sort the `idx_queue` by depth/name on every push anyway.
            dep_receiver.buffer_unordered(100),
            // Process nodes one-by-one and in order because processor is
            // effectively synchronous (if not for the mutexes) anyway.
            idx_receiver.buffered(1),
        )
        .try_for_each(|_| futures::future::ready(Ok(())))
        .await?;

        Ok(())
    }

    fn satisfy_dependency(
        graph: &mut Graph,
        node_dep: &NodeDependency,
    ) -> Result<bool, NodeMaintainerError> {
        if let Some(satisfier_idx) = graph.find_by_name(node_dep.node_idx, &node_dep.name)? {
            let requested = format!("{:}@{:}", node_dep.name, node_dep.spec).parse()?;
            if graph[satisfier_idx]
                .package
                .resolved()
                .satisfies(&requested)?
            {
                let edge_idx = graph.inner.add_edge(
                    node_dep.node_idx,
                    satisfier_idx,
                    Edge::new(requested.clone(), node_dep.dep_type.clone()),
                );
                graph[node_dep.node_idx]
                    .dependencies
                    .insert(node_dep.name.clone(), edge_idx);
                return Ok(true);
            }
            return Ok(false);
        }
        return Ok(false);
    }

    fn place_child(
        graph: &mut Graph,
        dependent_idx: NodeIndex,
        package: Package,
        dep_type: DepType,
    ) -> Result<NodeIndex, NodeMaintainerError> {
        let requested = package.from().clone();
        let child_name = UniCase::new(package.name().to_string());
        let child_node = Node::new(package);
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

        // Now we calculate the highest hierarchy location that we can place
        // this node in.
        let mut parent_idx = Some(dependent_idx);
        let mut target_idx = dependent_idx;
        'outer: while let Some(curr_target_idx) = parent_idx {
            if let Some(resolved) = graph.resolve_dep(curr_target_idx, &child_name) {
                for edge_ref in graph.inner.edges_directed(resolved, Direction::Incoming) {
                    let (from, _) = graph
                        .inner
                        .edge_endpoints(edge_ref.id())
                        .expect("Where did the edge go?!?!");
                    if graph.is_ancestor(curr_target_idx, from)
                        && !graph[resolved].package.resolved().satisfies(&requested)?
                    {
                        break 'outer;
                    }
                }
            }

            // No conflict yet. Let's try to go higher!
            target_idx = curr_target_idx;
            parent_idx = graph[curr_target_idx].parent;
        }

        // Finally, we put everything in its place.
        {
            let mut child_node = &mut graph[child_idx];
            // The parent is the _hierarchy_ location, so we set its parent
            // accordingly.
            child_node.parent = Some(target_idx);
        }
        {
            // Now we set backlinks: first, the dependent node needs to point
            // to the child, wherever it is in the graph.
            let dependent = &mut graph[dependent_idx];
            dependent.dependencies.insert(child_name.clone(), edge_idx);
        }
        {
            // Finally, we add the backlink from the parent node to the child.
            let node = &mut graph[target_idx];
            node.children.insert(child_name, child_idx);
        }
        Ok(child_idx)
    }

    fn package_deps<'b>(
        graph: &Graph,
        node_idx: NodeIndex,
        manifest: &'b CorgiManifest,
    ) -> Box<dyn Iterator<Item = ((&'b String, &'b String), DepType)> + 'b> {
        let deps = manifest
            .dependencies
            .iter()
            .map(|x| (x, DepType::Prod))
            .chain(
                manifest
                    .optional_dependencies
                    .iter()
                    .map(|x| (x, DepType::Opt)),
            )
            .chain(
                manifest
                    .peer_dependencies
                    .iter()
                    .map(|x| (x, DepType::Peer)),
            );

        if node_idx == graph.root {
            Box::new(deps.chain(manifest.dev_dependencies.iter().map(|x| (x, DepType::Dev))))
        } else {
            Box::new(deps)
        }
    }
}
