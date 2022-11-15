use std::cmp::Ordering;
use std::collections::{HashSet, VecDeque};
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use async_std::fs;
use futures::FutureExt;
use nassun::{Nassun, NassunOpts, Package};
use oro_common::Manifest;
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use unicase::UniCase;
use url::Url;

use crate::edge::{DepType, Edge};
use crate::error::NodeMaintainerError;
use crate::{Graph, Node};

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
        nm.resolve().await?;
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

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn render_to_file(&self, path: impl AsRef<Path>) -> Result<(), NodeMaintainerError> {
        fs::write(path.as_ref(), self.graph.render()).await?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn write_lockfile(&self, path: impl AsRef<Path>) -> Result<(), NodeMaintainerError> {
        fs::write(path.as_ref(), self.graph.to_kdl().to_string()).await?;
        Ok(())
    }

    pub fn to_resolved_tree(&self) -> crate::ResolvedTree {
        self.graph.to_resolved_tree()
    }

    pub fn to_kdl(&self) -> kdl::KdlDocument {
        self.graph.to_kdl()
    }

    pub fn render(&self) -> String {
        self.graph.render()
    }

    async fn resolve(&mut self) -> Result<(), NodeMaintainerError> {
        let mut packages = Vec::new();
        let mut q = VecDeque::new();
        q.push_back(self.graph.root);
        // Start iterating over the queue. We'll be adding things to it as we find them.
        while let Some(node_idx) = q.pop_front() {
            let mut names = HashSet::new();
            let manifest = self.graph[node_idx].package.metadata().await?.manifest;
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
                    let needs_new_node =
                        if let Some(satisfier_idx) = self.graph.find_by_name(node_idx, &name)? {
                            if self.graph[satisfier_idx]
                                .package
                                .resolved()
                                .satisfies(&requested)?
                            {
                                let edge_idx = self.graph.inner.add_edge(
                                    node_idx,
                                    satisfier_idx,
                                    Edge::new(requested, dep_type.clone()),
                                );
                                self.graph[node_idx]
                                    .dependencies
                                    .insert(name.clone(), edge_idx);
                                false
                            } else {
                                // The name does exist up our parent chain,
                                // but its resolution doesn't satisfy our
                                // request. We'll have to add a new node here.
                                true
                            }
                        } else {
                            true
                        };
                    if needs_new_node {
                        // Otherwise, we have to fetch package metadata to
                        // create a new node (which we'll place later).
                        packages.push(
                            self.nassun
                                .resolve(format!("{name}@{spec}"))
                                .map(|p| (p, dep_type)),
                        );
                    };
                    names.insert(name);
                }
            }

            // Order doesn't matter here: each node name is unique, so we
            // don't have to worry about races messing with placement.
            for (package, dep_type) in futures::future::join_all(packages.drain(..)).await {
                q.push_back(Self::place_child(
                    &mut self.graph,
                    node_idx,
                    package?,
                    dep_type,
                )?);
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
        Ok(())
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
        manifest: &'b Manifest,
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
