use std::cmp::Ordering;
use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::sync::Arc;

use async_std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use colored::Colorize;
use futures::{StreamExt, TryFutureExt};
use indexmap::IndexMap;
use nassun::client::Nassun;
use nassun::package::Package;
use nassun::PackageSpec;
use oro_common::{CorgiManifest, CorgiVersionMetadata};
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use unicase::UniCase;

#[cfg(not(target_arch = "wasm32"))]
use crate::error::IoContext;
use crate::error::NodeMaintainerError;
use crate::graph::{DepType, Edge, Graph, Node};
use crate::maintainer::{ProgressAdded, ProgressHandler};
#[cfg(not(target_arch = "wasm32"))]
use crate::META_FILE_NAME;
use crate::{Lockfile, LockfileNode};

#[derive(Debug, Clone)]
struct NodeDependency {
    name: UniCase<String>,
    spec: PackageSpec,
    dep_type: DepType,
    node_idx: NodeIndex,
}

pub(crate) struct Resolver<'a> {
    pub(crate) nassun: Nassun,
    pub(crate) graph: Graph,
    pub(crate) concurrency: usize,
    pub(crate) locked: bool,
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
                let maybe_spec = if let Some(mut fetches) = fetches.try_lock() {
                    if let Some(list) = fetches.get_mut(&dep.spec) {
                        // Package fetch is already in-flight, add dependency
                        // to the existing list.
                        list.push(dep);
                        None
                    } else {
                        // Fetch package since we are the first one to get here.
                        fetches.insert(dep.spec.clone(), vec![dep.clone()]);
                        Some(dep.spec.clone())
                    }
                } else {
                    // Mutex is locked - fetch the package
                    Some(dep.spec)
                };
                futures::future::ready(maybe_spec)
            })
            .filter_map(|maybe_spec| maybe_spec)
            .map(|spec| {
                self.nassun
                    .resolve_spec(spec.clone())
                    .map_ok(move |p| (p, spec))
            })
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

                    let dep = NodeDependency {
                        name: name.clone(),
                        spec,
                        dep_type,
                        node_idx,
                    };

                    if let Some(handler) = &self.on_resolution_added {
                        handler();
                    }

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
                                    &dep.spec,
                                )
                                .await?
                            {
                                let target_path = lockfile_node.path.clone();

                                let child_idx = Self::place_child(
                                    &mut self.graph,
                                    &dep,
                                    package,
                                    lockfile_node.into(),
                                    Some(target_path),
                                )?;
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
                                "deprecated".on_magenta(),
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

                            let child_idx = Self::place_child(
                                &mut self.graph,
                                &dep,
                                package.clone(),
                                manifest.clone(),
                                None,
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

        if self.locked {
            if let Some(lockfile) = lockfile {
                if lockfile != self.graph.to_lockfile()? {
                    return Err(NodeMaintainerError::LockfileMismatch);
                }
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

    fn satisfy_dependency(
        graph: &mut Graph,
        dep: &NodeDependency,
    ) -> Result<Option<NodeIndex>, NodeMaintainerError> {
        if let Some(satisfier_idx) = graph.find_by_name(dep.node_idx, &dep.name)? {
            if graph[satisfier_idx]
                .package
                .resolved()
                .satisfies(&dep.spec)?
            {
                let edge_idx = graph.inner.add_edge(
                    dep.node_idx,
                    satisfier_idx,
                    Edge::new(dep.spec.clone(), dep.dep_type),
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

    fn place_child(
        graph: &mut Graph,
        dep: &NodeDependency,
        package: Package,
        corgi: CorgiManifest,
        target_path: Option<Vec<UniCase<String>>>,
    ) -> Result<NodeIndex, NodeMaintainerError> {
        let child_name = &dep.name;
        let requested = &dep.spec;
        let dep_type = dep.dep_type;
        let dependent_idx = dep.node_idx;
        let child_node = Node::new(child_name.clone(), package, corgi, false)?;
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

        let target_path = target_path.map(|target| {
            let len = target.len();
            target.into_iter().take(len - 1).collect::<VecDeque<_>>()
        });
        let mut target_idx = dependent_idx;
        let mut parent_idx = Some(dependent_idx);
        'outer: while let Some(curr_target_idx) = parent_idx {
            if let Some(resolved) = graph.resolve_dep(curr_target_idx, child_name) {
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

            if let Some((req, _)) = graph[curr_target_idx].dependency_reqs.get(child_name) {
                if !graph[child_idx].package.resolved().satisfies(req)? {
                    break 'outer;
                }
            }

            // No conflict yet. Let's try to go higher!
            target_idx = curr_target_idx;
            parent_idx = graph[curr_target_idx].parent;

            // Try and place it where the lockfile asks us to, but only if it
            // makes sense for us to do so. If something's already there for
            // some reason, we'll ignore placing this.
            if let Some(locked) = &target_path {
                if locked == &graph.node_path(target_idx) {
                    break 'outer;
                }
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
            let child_node = &mut graph[child_idx];
            // The parent is the _hierarchy_ location, so we set its parent
            // accordingly.
            child_node.parent = Some(target_idx);
        }
        {
            // Finally, we add the backlink from the parent node to the child.
            let node = &mut graph[target_idx];
            if let Some(old) = node.children.insert(child_name.clone(), child_idx) {
                tracing::error!(
                    "clobbered {} at {} with {} (requested: {} by {}). This is a bug with the orogene resolver. Please report it.",
                    graph[old].package.resolved(),
                    graph.node_path_string(old),
                    graph[child_idx].package.resolved(),
                    requested,
                    graph.node_path_string(dependent_idx)
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
            async_std::fs::remove_file(&meta).await.io_context(|| {
                format!(
                    "Failed to remove Orogene meta file from node_modules, at {}",
                    meta.display()
                )
            })?;
        }
        Ok(())
    }
}
