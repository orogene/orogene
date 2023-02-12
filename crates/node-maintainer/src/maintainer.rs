use std::cmp::Ordering;
use std::collections::{HashSet, VecDeque};
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use async_std::fs;
use futures::{FutureExt, StreamExt};
use kdl::KdlDocument;
use nassun::{Nassun, NassunOpts, Package, PackageSpec};
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

    pub async fn from_manifest(self, manifest: CorgiManifest) -> Result<NodeMaintainer, NodeMaintainerError> {
        let nassun = self.nassun_opts.build();
        let package = Nassun::dummy_from_manifest(manifest);
        let mut nm = NodeMaintainer {
            nassun,
            graph: Default::default(),
        };
        let node = nm.graph.inner.add_node(Node::new(package));
        nm.graph[node].root = node;
        nm.run_resolver().await?;
        Ok(nm)
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

    pub async fn package_at_path(
        &self,
        path: &Path,
    ) -> Result<Option<Package>, NodeMaintainerError> {
        self.graph.package_at_path(path).await
    }

    async fn run_resolver(&mut self) -> Result<(), NodeMaintainerError> {
        let mut package_streams = futures::stream::select_all::SelectAll::new();
        let mut q = VecDeque::new();
        q.push_back(self.graph.root);
        // Start iterating over the queue. We'll be adding things to it as we find them.
        while !q.is_empty() || !package_streams.is_empty() {
            while let Some(node_idx) = q.pop_front() {
                let mut names = HashSet::new();
                let mut packages = Vec::new();
                let manifest = self.graph[node_idx]
                    .package
                    .corgi_metadata()
                    .await?
                    .manifest;
                // Grab all the deps from the current package and fire off a
                // lookup. These will be resolved concurrently.
                for ((name, spec), dep_type) in self.package_deps(node_idx, &manifest) {
                    // `dependencies` > `optionalDependencies` ->
                    // `peerDependencies` -> `devDependencies` (if we're looking
                    // at root)
                    let name = UniCase::new(name.clone());
                    if !names.contains(&name) {
                        let requested = format!("{name}@{spec}").parse()?;
                        // Walk up the current hierarchy to see if we find a
                        // dependency that already satisfies this request. If so,
                        // make a new edge and move on.
                        if !Self::satisfy_dependency(
                            &mut self.graph,
                            node_idx,
                            &dep_type,
                            &name,
                            &requested,
                        )? {
                            // Otherwise, we have to fetch package metadata to
                            // create a new node (which we'll place later).
                            packages.push(
                                self.nassun
                                    .resolve(format!("{name}@{spec}"))
                                    .map(move |p| (p, requested, dep_type, node_idx)),
                            );
                        };
                        names.insert(name);
                    }
                }

                package_streams.push(futures::stream::iter(packages).buffer_unordered(30));
            }

            // Order doesn't matter here: each node name is unique, so we
            // don't have to worry about races messing with placement.
            if let Some((package, requested, dep_type, dependent_idx)) = package_streams.next().await {
                let package = package?;
                let name = UniCase::new(package.name().to_string());
                if Self::satisfy_dependency(
                    &mut self.graph,
                    dependent_idx,
                    &dep_type,
                    &name,
                    &requested,
                )? {
                    continue;
                }
                q.push_back(Self::place_child(
                    &mut self.graph,
                    dependent_idx,
                    package,
                    dep_type,
                )?);

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
        dependent_idx: NodeIndex,
        dep_type: &DepType,
        name: &UniCase<String>,
        requested: &PackageSpec,
    ) -> Result<bool, NodeMaintainerError> {
        if let Some(satisfier_idx) = graph.find_by_name(dependent_idx, &name)? {
            if graph[satisfier_idx]
                .package
                .resolved()
                .satisfies(&requested)?
            {
                let edge_idx = graph.inner.add_edge(
                    dependent_idx,
                    satisfier_idx,
                    Edge::new(requested.clone(), dep_type.clone()),
                );
                graph[dependent_idx]
                    .dependencies
                    .insert(name.clone(), edge_idx);
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

    fn package_deps<'a, 'b>(
        &'a self,
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

        if node_idx == self.graph.root {
            Box::new(deps.chain(manifest.dev_dependencies.iter().map(|x| (x, DepType::Dev))))
        } else {
            Box::new(deps)
        }
    }
}
