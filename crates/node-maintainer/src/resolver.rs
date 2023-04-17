use std::cmp::Ordering;
use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::sync::Arc;

use async_std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use colored::Colorize;
use futures::StreamExt;
use indexmap::IndexMap;
use nassun::client::Nassun;
use nassun::package::Package;
use nassun::PackageSpec;
use oro_common::{CorgiManifest, CorgiVersionMetadata};
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use unicase::UniCase;

use crate::error::NodeMaintainerError;
use crate::graph::{DepType, Edge, Graph, Node};
#[cfg(not(target_arch = "wasm32"))]
use crate::META_FILE_NAME;
use crate::{Lockfile, LockfileNode, ProgressAdded, ProgressHandler};

#[derive(Debug, Clone)]
struct NodeDependency {
    name: UniCase<String>,
    requested: PackageSpec,
    dep_type: DepType,
    resolved: Option<(Package, LockfileNode)>,
    node_idx: NodeIndex,
}

pub(crate) struct Resolver<'a> {
    pub(crate) nassun: Nassun,
    pub(crate) graph: Graph,
    pub(crate) concurrency: usize,
    #[allow(dead_code)]
    pub(crate) root: &'a Path,
    pub(crate) actual_tree: Option<Lockfile>,
    pub(crate) on_resolution_added: Option<ProgressAdded>,
    pub(crate) on_resolve_progress: Option<ProgressHandler>,
}

impl<'a> Resolver<'a> {
    pub(crate) async fn run_resolver(
        mut self,
        lockfile: Option<Lockfile>,
    ) -> Result<(Graph, Option<Lockfile>), NodeMaintainerError> {
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
        let fetches: IndexMap<PackageSpec, Vec<NodeDependency>> = IndexMap::new();
        let fetches = Arc::new(Mutex::new(fetches));

        let mut package_stream = package_stream
            .map(|dep: NodeDependency| {
                if let Some(mut fetches) = fetches.try_lock() {
                    if let Some(list) = fetches.get_mut(&dep.requested) {
                        // Package fetch is already in-flight, add dependency
                        // to the existing list.
                        list.push(dep);
                        return futures::future::ready(None);
                    } else {
                        // Fetch package since we are the first one to get here.
                        fetches.insert(dep.requested.clone(), vec![dep.clone()]);
                    }
                };
                futures::future::ready(Some(dep))
            })
            .filter_map(|dep| dep)
            .map(|dep| Self::resolve_dep(&self.nassun, dep))
            .buffer_unordered(self.concurrency)
            .ready_chunks(self.concurrency);

        // Start iterating over the queue. We'll be adding things to it as we find them.
        while !q.is_empty() || in_flight != 0 {
            while let Some(node_idx) = q.pop_front() {
                let mut names = HashSet::new();
                // Grab all the deps from the current package and fire off a
                // lookup. These will be resolved concurrently.
                for (name, (spec, dep_type)) in self.graph[node_idx].dependency_reqs.clone() {
                    if names.contains(&name) {
                        continue;
                    } else {
                        names.insert(name.clone());
                    }

                    let mut dep = NodeDependency {
                        name: name.clone(),
                        requested: spec,
                        dep_type,
                        node_idx,
                        resolved: None,
                    };

                    if let Some(handler) = &self.on_resolution_added {
                        handler();
                    }

                    // Walk up the current hierarchy to see if we find a
                    // dependency that already satisfies this request. If so,
                    // make a new edge and move on.
                    if let Some(_child_idx) = Self::satisfy_dependency(&mut self.graph, &dep)? {
                        if let Some(handler) = &self.on_resolve_progress {
                            handler(&self.graph[_child_idx].package);
                        }
                    } else {
                        // If we have a lockfile, first check if there's a
                        // dep there that would satisfy this.
                        let lock = if lockfile.is_some() {
                            &lockfile
                        } else {
                            // Fall back to the actual tree lock if it's there.
                            &self.actual_tree
                        };
                        if let Some(lockfile) = lock {
                            if let Some((package, lockfile_node)) = self
                                .satisfy_from_lockfile(
                                    &self.graph,
                                    node_idx,
                                    lockfile,
                                    &name,
                                    &dep.requested,
                                )
                                .await?
                            {
                                dep.resolved = Some((package, lockfile_node));
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
                    let (package, lockfile_node, spec) = res?;
                    let deps = fetches.lock().await.remove(&spec);

                    if let Some(deps) = deps {
                        in_flight -= deps.len();

                        let (target_path, manifest) = if let Some((target_path, manifest)) =
                            lockfile_node.map(|n| (Some(n.path.clone()), n.into()))
                        {
                            (target_path, manifest)
                        } else {
                            // For performance reasons, we skip deprecation
                            // checks for lockfile dependencies.
                            let CorgiVersionMetadata {
                                manifest,
                                deprecated,
                                ..
                            } = package.corgi_metadata().await?;

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
                            (None, manifest)
                        };

                        for dep in deps {
                            if let Some(_child_idx) =
                                Self::satisfy_dependency(&mut self.graph, &dep)?
                            {
                                if let Some(handler) = &self.on_resolve_progress {
                                    handler(&self.graph[_child_idx].package);
                                }
                                continue;
                            }

                            let child_idx = Self::place_child(
                                &mut self.graph,
                                dep.node_idx,
                                package.clone(),
                                &dep.requested,
                                dep.dep_type,
                                manifest.clone(),
                                target_path.clone(),
                            )?;

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
        Ok((self.graph, self.actual_tree))
    }

    async fn resolve_dep(
        nassun: &Nassun,
        dep: NodeDependency,
    ) -> Result<(Package, Option<LockfileNode>, PackageSpec), NodeMaintainerError> {
        if let Some((package, lockfile_node)) = dep.resolved {
            Ok((package, Some(lockfile_node), dep.requested.clone()))
        } else {
            let pkg = nassun.resolve_spec(&dep.requested).await?;
            Ok((pkg, None, dep.requested.clone()))
        }
    }

    fn satisfy_dependency(
        graph: &mut Graph,
        dep: &NodeDependency,
    ) -> Result<Option<NodeIndex>, NodeMaintainerError> {
        if let Some(satisfier_idx) = graph.resolve_dep(dep.node_idx, &dep.name) {
            if graph[satisfier_idx]
                .package
                .resolved()
                .satisfies(&dep.requested)?
            {
                let edge_idx = graph.inner.add_edge(
                    dep.node_idx,
                    satisfier_idx,
                    Edge::new(dep.requested.clone(), dep.dep_type),
                );
                graph[dep.node_idx]
                    .dependencies
                    .insert(dep.name.clone(), edge_idx);
                return Ok(Some(satisfier_idx));
            }
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
                        // TODO: Log this
                        //
                        // We found a lockfile node in a place
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

    fn check_placement(
        graph: &mut Graph,
        target: NodeIndex,
        requested: &PackageSpec,
        child_name: &UniCase<String>,
    ) -> Result<bool, NodeMaintainerError> {
        if let Some(resolved) = graph.resolve_dep(target, child_name) {
            for edge_ref in graph.inner.edges_directed(resolved, Direction::Incoming) {
                let (from, _) = graph
                    .inner
                    .edge_endpoints(edge_ref.id())
                    .expect("Where did the edge go?!?!");
                if graph.is_ancestor(target, from)
                    && !graph[resolved].package.resolved().satisfies(requested)?
                {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn place_child(
        graph: &mut Graph,
        dependent_idx: NodeIndex,
        package: Package,
        requested: &PackageSpec,
        dep_type: DepType,
        corgi: CorgiManifest,
        target_path: Option<Vec<UniCase<String>>>,
    ) -> Result<NodeIndex, NodeMaintainerError> {
        let child_name = UniCase::new(package.name().to_string());
        let child_node = Node::new(package, corgi, false)?;
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
                if let Some(new_target) = graph[target_idx].children.get(segment) {
                    target_idx = *new_target;
                } else {
                    // We couldn't find the target path. We'll just place the
                    // node in the highest possible location.
                    found_in_path = false;
                    break;
                }
            }
            if found_in_path {
                found_in_path = Self::check_placement(graph, target_idx, requested, &child_name)?;
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
            if let Some(old) = node.children.insert(child_name, child_idx) {
                // TODO: this is probably an error?
                println!(
                    "clobbered {:?} with {:?}",
                    graph[old].package.resolved(),
                    graph[child_idx].package.resolved()
                );
            }
        }
        Ok(child_idx)
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
