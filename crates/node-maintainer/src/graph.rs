use std::{
    collections::{HashMap, HashSet, VecDeque},
    ops::{Index, IndexMut},
    path::Path,
};

use kdl::KdlDocument;
use nassun::{Package, PackageResolution};
use petgraph::{
    dot::Dot,
    stable_graph::{EdgeIndex, NodeIndex, StableGraph},
    visit::EdgeRef,
    Direction,
};
use unicase::UniCase;

use crate::{DepType, Edge, Lockfile, LockfileNode, Node, NodeMaintainerError};

#[derive(Debug, Hash, PartialEq, Eq)]
pub(crate) struct DemotionTarget {
    /// Index of the target ancestor node that should hold the demoted copy.
    pub(crate) target_idx: NodeIndex,

    /// Index of the dependent node
    pub(crate) dependent_idx: NodeIndex,

    /// Index of the edge between dependency and dependent
    pub(crate) edge_idx: EdgeIndex,
}

#[derive(Debug, Default)]
pub struct Graph {
    pub(crate) root: NodeIndex,
    pub(crate) inner: StableGraph<Node, Edge>,
}

impl Index<NodeIndex> for Graph {
    type Output = Node;

    fn index(&self, index: NodeIndex) -> &Self::Output {
        &self.inner[index]
    }
}

impl IndexMut<NodeIndex> for Graph {
    fn index_mut(&mut self, index: NodeIndex) -> &mut Self::Output {
        &mut self.inner[index]
    }
}

impl Index<EdgeIndex> for Graph {
    type Output = Edge;

    fn index(&self, index: EdgeIndex) -> &Self::Output {
        &self.inner[index]
    }
}

impl IndexMut<EdgeIndex> for Graph {
    fn index_mut(&mut self, index: EdgeIndex) -> &mut Self::Output {
        &mut self.inner[index]
    }
}

impl Graph {
    pub fn resolve_dep(&self, node: NodeIndex, dep: &UniCase<String>) -> Option<NodeIndex> {
        let mut current = Some(node);
        while let Some(curr) = current {
            if let Some(resolved) = self[curr].children.get(dep) {
                return Some(*resolved);
            }
            current = self[curr].parent;
        }
        None
    }

    pub fn is_ancestor(&self, ancestor: NodeIndex, descendant: NodeIndex) -> bool {
        let mut current = Some(descendant);
        while let Some(curr) = current {
            if curr == ancestor {
                return true;
            }
            current = self[curr].parent;
        }
        false
    }

    pub fn to_lockfile(&self) -> Result<Lockfile, NodeMaintainerError> {
        let root = self.node_lockfile_node(self.root, true)?;
        let packages = self
            .inner
            .node_indices()
            .filter(|idx| *idx != self.root)
            .map(|idx| {
                let node = self.node_lockfile_node(idx, false)?;
                Ok((
                    UniCase::from(
                        node.path
                            .iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                            .join("/node_modules/"),
                    ),
                    node,
                ))
            })
            .collect::<Result<HashMap<_, _>, NodeMaintainerError>>()?;
        Ok(Lockfile {
            version: 1,
            root,
            packages,
        })
    }

    pub fn to_kdl(&self) -> Result<KdlDocument, NodeMaintainerError> {
        Ok(self.to_lockfile()?.to_kdl())
    }

    pub fn render(&self) -> String {
        format!(
            "{:?}",
            Dot::new(&self.inner.map(
                |_, mut node| {
                    let resolved = node.package.resolved();
                    let mut label = node.package.name().to_string();
                    while let Some(node_idx) = &node.parent {
                        node = &self.inner[*node_idx];
                        let name = node.package.name();
                        label = format!("{name}/node_modules/{label}");
                    }
                    format!("{resolved:?} @ {label}")
                },
                |_, edge| { format!("{}", edge.requested) }
            ))
        )
    }

    pub(crate) async fn package_at_path(
        &self,
        path: &Path,
    ) -> Result<Option<Package>, NodeMaintainerError> {
        let mut current = Some(self.root);
        let mut in_nm = true;
        let nm = UniCase::new("node_modules".to_owned());
        for segment in path {
            let segment = UniCase::new(segment.to_string_lossy().into());
            if segment == nm {
                in_nm = true;
                continue;
            } else if let Some(curr_idx) = current {
                if !in_nm {
                    break;
                } else if let Some(child) = self.inner[curr_idx].children.get(&segment) {
                    current = Some(*child);
                    in_nm = false;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(current.map(|idx| self.inner[idx].package.clone()))
    }

    pub(crate) fn find_by_name(
        &self,
        parent: NodeIndex,
        name: &UniCase<String>,
    ) -> Result<Option<NodeIndex>, NodeMaintainerError> {
        let mut parent = self.inner.node_weight(parent);
        while let Some(node) = parent {
            if node.children.contains_key(name) {
                return Ok(Some(node.children[name]));
            }
            parent = node.parent.and_then(|idx| self.inner.node_weight(idx));
        }
        Ok(None)
    }

    pub(crate) fn node_path(&self, node_idx: NodeIndex) -> VecDeque<UniCase<String>> {
        let node = &self.inner[node_idx];
        let mut path = VecDeque::new();
        path.push_front(UniCase::new(node.package.name().to_owned()));
        if node_idx != self.root {
            let mut parent = node.parent;
            while let Some(parent_idx) = parent {
                if parent_idx == self.root {
                    break;
                }
                path.push_front(UniCase::new(
                    self.inner[parent_idx].package.name().to_owned(),
                ));
                parent = self.inner[parent_idx].parent;
            }
        };
        path
    }

    /// True if `idx` is a direct dependency of `parent`.
    pub(crate) fn is_dependency(&self, parent: NodeIndex, idx: NodeIndex) -> bool {
        self.inner.contains_edge(parent, idx)
    }

    /// A vector of Node's children that could be used for its demotion
    /// placement (moving it deeper into the tree).
    pub(crate) fn get_demotion_targets(&self, idx: NodeIndex) -> Vec<DemotionTarget> {
        let dependents = self.inner.edges_directed(idx, Direction::Incoming);

        let mut targets = HashSet::new();
        for d in dependents {
            let mut current = d.source();

            while let Some(parent_idx) = self.inner[current].parent {
                if parent_idx != idx {
                    current = parent_idx
                }
            }

            targets.insert(DemotionTarget {
                target_idx: current,
                dependent_idx: d.source(),
                edge_idx: d.id(),
            });
        }

        return targets.drain().collect();
    }

    /// True if every Node's sub-dependency either:
    /// - Has a parent whose is an ancestor of Node's parent
    /// - Within Node's subtree (including the node itself)
    ///
    /// If either condition are true for every sub-dependency - node could be
    /// made sibling of its parent (promoted).
    pub(crate) fn can_be_promoted(&self, node: NodeIndex) -> bool {
        let node_parent = self.inner[node].parent.expect("parent of non-root node");
        let mut q = VecDeque::new();
        q.push_back(node);

        while let Some(dep) = q.pop_front() {
            q.extend(self.inner.neighbors_directed(dep, Direction::Outgoing));

            if dep == node {
                continue;
            }

            if let Some(dep_parent) = self.inner[dep].parent {
                // Dependency is a sibling of node, can't promote
                if dep_parent == node_parent {
                    return false;
                }

                if !self.is_ancestor(dep_parent, node_parent) && !self.is_ancestor(node, dep) {
                    return false;
                }
            }
        }

        true
    }

    /// Merge `deduped` nodes into a single node and put it as child of `target`
    pub(crate) fn merge_and_promote(&mut self, target: NodeIndex, deduped: &[NodeIndex]) {
        // Take the first node from the `deduped` list.
        let mut deduped = deduped.iter();
        let promoted_idx = deduped
            .next()
            .expect("at least one item in deduped list")
            .to_owned();

        // Disconnect the node from its current parent.
        let name = UniCase::new(self.inner[promoted_idx].package.name().to_string());
        if let Some(parent) = self.inner[promoted_idx].parent {
            let removed = self.inner[parent].children.remove(&name);
            assert_eq!(removed, Some(promoted_idx));
        }

        // Promote the node by placing it as the child of the `target`.
        self.inner[target].children.insert(name, promoted_idx);
        self.inner[promoted_idx].parent = Some(target);

        // Remove the rest of `deduped` from the graph
        let mut dependents = Vec::new();
        for &idx in deduped {
            // Move all dependents to the saved node.
            dependents.extend(
                self.inner
                    .edges_directed(idx, Direction::Incoming)
                    .map(|r| (r.source(), r.id())),
            );
            for (source_idx, edge_idx) in dependents.drain(..) {
                if let Some(edge) = self.inner.remove_edge(edge_idx) {
                    self.inner.add_edge(source_idx, promoted_idx, edge);
                }
            }

            // Remove the deduped node and its children
            self.remove_subtree(idx);
        }
    }

    /// Clone `node` and put it as a child of each target in `targets`.
    pub(crate) fn clone_and_demote(&mut self, node: NodeIndex, targets: &[DemotionTarget]) {
        // Get and remove the edges between node and its dependents
        // before the node itself is removed. We will reconstruct these edges
        // when we clone and place the node.
        let targets_and_edges = targets
            .iter()
            .filter_map(|t| self.inner.remove_edge(t.edge_idx).map(|edge| (t, edge)))
            .collect::<Vec<_>>();

        let mut target_to_cloned: HashMap<NodeIndex, NodeIndex> = HashMap::new();

        let name = self.inner[node].package.name().to_string();

        // For each target ancestor node + (dependent -> node) edge
        for (target, edge) in targets_and_edges {
            // Create or use previously created demoted clone.
            let cloned_idx = match target_to_cloned.get(&target.target_idx) {
                Some(cloned_idx) => *cloned_idx,
                None => {
                    // Create a new node which is a clone of the original one
                    let cloned_idx = self.clone_subtree(node);

                    // Insert it into the placement target
                    self.inner[cloned_idx].parent = Some(target.target_idx);
                    self.inner[target.target_idx]
                        .children
                        .insert(UniCase::new(name.clone()), cloned_idx);

                    // Cache the node so that we don't duplicate it
                    target_to_cloned.insert(target.target_idx, cloned_idx);

                    cloned_idx
                }
            };

            // Reconnect the edge from dependent to the clone.
            self.inner.add_edge(target.dependent_idx, cloned_idx, edge);
        }

        // When we are done with the original node - remove its subtree from
        // the graph completely.
        self.remove_subtree(node);
    }

    fn remove_subtree(&mut self, node: NodeIndex) -> Option<Node> {
        // Detach node from its parent
        if let Some(parent_idx) = self.inner[node].parent {
            let parent = &mut self.inner[parent_idx];

            parent.children.retain(|_, child_idx| *child_idx != node);
        }

        let mut result = None;

        // Walk through its children remove them from the graph
        // (This automatically removes edges)
        let mut q = VecDeque::new();
        q.push_back(node);
        while let Some(node) = q.pop_front() {
            let node = self
                .inner
                .remove_node(node)
                .expect("removed node to be present");

            q.extend(node.children.values());
            result.get_or_insert(node);

            // Note that we don't bother with parent/children properties because
            // whole subtree gets removed from the graph.
        }

        // Return original node
        result
    }

    fn clone_subtree(&mut self, node: NodeIndex) -> NodeIndex {
        let mut clone_idxs: HashMap<NodeIndex, NodeIndex> = HashMap::new();

        // Walk subtree and clone each node, remembering the new indexes.
        let mut q = VecDeque::new();
        q.push_back(node);
        while let Some(idx) = q.pop_front() {
            let node = &self.inner[idx];
            let parent = node.parent;
            q.extend(node.children.values());

            let clone_idx = self.inner.add_node(node.clone());
            self.inner[clone_idx].idx = clone_idx;
            self.inner[clone_idx].parent = None;
            self.inner[clone_idx].children.clear();
            clone_idxs.insert(idx, clone_idx);

            // Reconnect cloned nodes within the subtree.
            let parent_idx = parent.and_then(|idx| clone_idxs.get(&idx)).copied();
            self.inner[clone_idx].parent = parent_idx;
            if let Some(parent_idx) = parent_idx {
                let name = UniCase::new(self.inner[idx].package.name().to_string());
                self.inner[parent_idx].children.insert(name, clone_idx);
            }
        }

        // Now that we have a subtree structure - restore edges between
        // dependents/dependencies of the cloned node.
        let mut edges = Vec::new();
        for (&original_idx, &clone_idx) in &clone_idxs {
            edges.extend(
                self.inner
                    .edges_directed(original_idx, Direction::Incoming)
                    .map(|e| {
                        // Note that existing source might be within the cloned
                        // subtree.
                        let source = clone_idxs.get(&e.source()).copied().unwrap_or(e.source());
                        (source, clone_idx, e.weight().clone())
                    }),
            );

            edges.extend(
                self.inner
                    .edges_directed(original_idx, Direction::Outgoing)
                    .map(|e| {
                        // Note that existing target might be within the cloned
                        // subtree.
                        let target = clone_idxs.get(&e.target()).copied().unwrap_or(e.target());
                        (clone_idx, target, e.weight().clone())
                    }),
            );

            for (source, target, weight) in edges.drain(..) {
                self.inner.add_edge(source, target, weight);
            }
        }

        clone_idxs
            .remove(&node)
            .expect("root of subtree to be cloned")
    }

    fn node_lockfile_node(
        &self,
        node: NodeIndex,
        is_root: bool,
    ) -> Result<LockfileNode, NodeMaintainerError> {
        let path = self.node_path(node);
        let node = &self.inner[node];
        let resolved = match node.package.resolved() {
            PackageResolution::Npm { tarball, .. } => tarball.to_string(),
            PackageResolution::Dir { path, .. } => path.to_string_lossy().into(),
            PackageResolution::Git { info, .. } => info.to_string(),
        };
        let version = if let PackageResolution::Npm { version, .. } = node.package.resolved() {
            Some(version.clone())
        } else {
            None
        };

        let mut prod_deps = HashMap::new();
        let mut dev_deps = HashMap::new();
        let mut peer_deps = HashMap::new();
        let mut opt_deps = HashMap::new();
        for e in self.inner.edges_directed(node.idx, Direction::Outgoing) {
            use DepType::*;

            let name = self.inner[e.target()].package.name();
            let edge = e.weight();

            let deps = match edge.dep_type {
                Prod => &mut prod_deps,
                Dev => &mut dev_deps,
                Peer => &mut peer_deps,
                Opt => &mut opt_deps,
            };
            deps.insert(name.to_string(), edge.requested.requested().clone());
        }
        Ok(LockfileNode {
            name: UniCase::new(node.package.name().to_string()),
            is_root,
            path: path.into(),
            resolved: Some(resolved),
            version,
            dependencies: prod_deps,
            dev_dependencies: dev_deps,
            peer_dependencies: peer_deps,
            optional_dependencies: opt_deps,
            integrity: match node.package.resolved() {
                PackageResolution::Npm { ref integrity, .. } => integrity.clone(),
                _ => None,
            },
        })
    }
}
