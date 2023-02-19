use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsStr,
    ops::{Index, IndexMut},
    path::Path,
};

use kdl::KdlDocument;
use nassun::{Package, PackageResolution};
use petgraph::{
    dot::Dot,
    stable_graph::{EdgeIndex, NodeIndex, StableGraph},
};
use unicase::UniCase;

use crate::{DepType, Edge, Lockfile, LockfileNode, Node, NodeMaintainerError};
#[cfg(debug_assertions)]
use NodeMaintainerError::GraphValidationError;

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
            .collect::<Result<BTreeMap<_, _>, NodeMaintainerError>>()?;
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

    pub fn render_tree(&self) -> String {
        let mut res = Vec::new();
        res.push("digraph {".into());
        for node in self.inner.node_weights() {
            let label = format!("{:?} @ {}", node.package.resolved(), node.package.name());
            res.push(format!("  {} [label={:?}]", node.idx.index(), label));

            if let Some(parent) = node.parent {
                res.push(format!("  {} -> {}", parent.index(), node.idx.index()));
            }
        }
        res.push("}".into());
        res.join("\n")
    }

    pub(crate) fn package_at_path(
        &self,
        path: &Path,
    ) -> Result<Option<Package>, NodeMaintainerError> {
        let mut current = Some(self.root);
        let mut in_nm = true;
        let slash = OsStr::new("/");
        let backslash = OsStr::new("\\");
        let nm = UniCase::new("node_modules".to_owned());
        for raw_segment in path {
            let segment = UniCase::new(raw_segment.to_string_lossy().into());
            if segment == nm || slash == raw_segment || backslash == raw_segment {
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

    /// Validate that file system hierarchy (parent -> children) is compatible
    /// with graph edges (dependent -> dependency).
    #[cfg(debug_assertions)]
    pub(crate) fn validate(&self) -> Result<(), NodeMaintainerError> {
        // Verify that all nodes in the tree are in the graph
        let mut q = VecDeque::new();
        q.push_back(self.root);
        while let Some(node) = q.pop_front() {
            if !self.inner.contains_node(node) {
                return Err(GraphValidationError(format!(
                    "Missing node in the graph for: {node:?}"
                )));
            }

            q.extend(self.inner[node].children.values());
        }

        // Verify that directed graph makes all dependencies available to
        // dependents.
        for edge_idx in self.inner.edge_indices() {
            let (dependent, dependency) = self
                .inner
                .edge_endpoints(edge_idx)
                .ok_or(GraphValidationError(format!("Missing edge: {edge_idx:?}")))?;

            let dependent = &self.inner[dependent];
            let dependency = &self.inner[dependency];

            let edge = self
                .inner
                .edge_weight(edge_idx)
                .ok_or(GraphValidationError(format!(
                    "Missing edge weight: {edge_idx:?}"
                )))?;

            let dependency_parent = dependency.parent.ok_or(GraphValidationError(format!(
                "Missing dependency parent: {:?}",
                dependent.package.resolved(),
            )))?;

            // Check parent->child relationship
            let dependency_name = UniCase::new(dependency.package.name().into());
            if !self.inner[dependency_parent]
                .children
                .contains_key(&dependency_name)
            {
                return Err(GraphValidationError(format!(
                    "Dependency {:?} is not in the children of {:?}",
                    dependency.package.resolved(),
                    self.inner[dependency_parent]
                )));
            }

            // Parent of the dependency should be an ancestor of the dependent
            // or dependency should be in dependent's subtree.
            if !self.is_ancestor(dependent.idx, dependency.idx)
                && !self.is_ancestor(dependency_parent, dependent.idx)
            {
                return Err(GraphValidationError(format!(
                    "Dependency {:?} is unreachable from {:?}",
                    dependency.package.resolved(),
                    dependent.package.resolved(),
                )));
            }

            // Check that dependency satisfies the requirement
            if !dependency.package.resolved().satisfies(&edge.requested)? {
                return Err(GraphValidationError(format!(
                    "Dependency {:?} does not satisfy requirement {:?} from {:?}",
                    dependency.package.resolved(),
                    edge.requested,
                    dependent.package.resolved(),
                )));
            }
        }

        Ok(())
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

        let mut prod_deps = BTreeMap::new();
        let mut dev_deps = BTreeMap::new();
        let mut peer_deps = BTreeMap::new();
        let mut opt_deps = BTreeMap::new();
        let dependencies = node
            .dependencies
            .iter()
            .map(|(name, edge_idx)| {
                let edge = &self.inner[*edge_idx];
                (name, &edge.requested, &edge.dep_type)
            });
        for (name, requested, dep_type) in dependencies {
            use DepType::*;
            let deps = match dep_type {
                Prod => &mut prod_deps,
                Dev => &mut dev_deps,
                Peer => &mut peer_deps,
                Opt => &mut opt_deps,
            };
            deps.insert(name.to_string(), requested.requested().clone());
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
