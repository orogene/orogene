use std::collections::{HashSet, VecDeque};
use std::path::Path;

use async_std::fs;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use nassun::{Nassun, NassunOpts, Package};
use oro_common::Manifest;
use petgraph::stable_graph::NodeIndex;
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

    pub async fn render_to_file(&self, path: impl AsRef<Path>) -> Result<(), NodeMaintainerError> {
        fs::write(path.as_ref(), self.graph.render()).await?;
        Ok(())
    }

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
        let mut packages = FuturesUnordered::new();
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
            // We drain the current contents of our FuturesUnordered that we
            // added all those lookups to. Next items will be in whatever
            // order resolves first.
            while let Some((package, dep_type)) = packages.next().await {
                q.push_back(Self::place_child(
                    &mut self.graph,
                    node_idx,
                    package?,
                    dep_type,
                ));
            }
        }
        Ok(())
    }

    fn place_child(
        graph: &mut Graph,
        dependent_idx: NodeIndex,
        package: Package,
        dep_type: DepType,
    ) -> NodeIndex {
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
            Edge::new(graph[child_idx].package.from().clone(), dep_type),
        );

        // Now we calculate the highest hierarchy location that we can place
        // this node in.
        let mut parent_idx = Some(dependent_idx);
        let mut target_idx = dependent_idx;
        while let Some(curr_target_idx) = parent_idx {
            if graph[curr_target_idx].children.contains_key(&child_name) {
                // We've run into a conflict, so we can't place it in this
                // parent. We previously checked if this conflict would have
                // satisfied our request, so there's no need to worry about
                // that at this point.
                break;
            }

            // TODO: this is wrong, it doesn't take into account trees like this:
            //
            // - A1
            // - B
            //   +- C
            //      +- D (dep on A1)
            //   +- F (dep on A2)
            //
            // In this case, when trying to place A2, we look at our parent
            // (B), and see that it doesn't depend on A at all, so we look one
            // more up and notice the conflict with A1. So, we place the
            // dependency inside B:
            //
            // - A1
            // - B
            //   +- C
            //      +- D (dep on A1)
            //   +- F (dep on A2)
            //   +- A2
            //
            //
            // But this is wrong, because now D will resolve A to A2 instead
            // of A1! NPM Classic got around this by placing "phantom"
            // dependency crumbs along the way whenever it "bubbled up"
            // dependencies in the tree.
            //
            // I'd like to be able to pull this off without needing those
            // phantom deps. I don't think any of the other resolvers do it?
            if let Some(parent_idx) = graph[curr_target_idx].parent {
                // Check the current target's parent to see if any of _its_
                // currently-placed dependencies would be shadowed by us
                // placing our new node as a sibling or below it.
                if let Some(edge) = graph[parent_idx].dependencies.get(&child_name) {
                    if graph
                        .inner
                        .edge_endpoints(*edge)
                        .map(|(from, _)| {
                            graph[from].depth(graph) >= graph[curr_target_idx].depth(graph)
                        })
                        .expect("edge missing nodes?")
                    {
                        // We've placed a dependency in an ancestor that would
                        // be shadowed by our placement, so we can't place it
                        // at this level. Stop bubbling up.
                        break;
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
        child_idx
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
