use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsStr,
    ops::{Index, IndexMut},
    path::Path,
};

use kdl::KdlDocument;
use nassun::{package::Package, PackageResolution};
use petgraph::{
    dot::Dot,
    stable_graph::{EdgeIndex, NodeIndex, StableGraph},
};
use unicase::UniCase;

use crate::{error::NodeMaintainerError, DepType, Edge, Lockfile, LockfileNode, Node};

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
        for parent in self.node_parent_iter(node) {
            if let Some(resolved) = parent.children.get(dep) {
                return Some(*resolved);
            }
        }
        None
    }

    pub fn is_ancestor(&self, ancestor: NodeIndex, descendant: NodeIndex) -> bool {
        self.node_parent_iter(descendant)
            .any(|parent| parent.idx == ancestor)
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

    pub(crate) fn node_parent_iter(&self, idx: NodeIndex) -> NodeParentIterator {
        NodeParentIterator {
            graph: self,
            current: Some(idx),
        }
    }

    pub(crate) fn node_at_path(&self, path: &Path) -> Option<&Node> {
        let mut current = Some(self.root);
        let mut in_nm = true;
        let mut scope = None;
        let slash = OsStr::new("/");
        let backslash = OsStr::new("\\");
        let nm = UniCase::new("node_modules".to_owned());
        for raw_segment in path {
            let str_segment = raw_segment.to_string_lossy().to_string();
            let segment = UniCase::new(str_segment.clone());
            if (segment == nm && scope.is_none())
                || slash == raw_segment
                || backslash == raw_segment
            {
                in_nm = true;
                continue;
            } else if let Some(curr_idx) = current {
                if !in_nm {
                    break;
                } else if segment.starts_with('@') {
                    scope = Some(segment.to_string());
                } else if let Some(curr_scope) = scope.as_deref() {
                    let scoped_seg = UniCase::new(format!("{curr_scope}/{segment}"));
                    if let Some(child) = self.inner[curr_idx].children.get(&scoped_seg) {
                        current = Some(*child);
                    }
                    in_nm = false;
                    scope = None;
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
        if current == Some(self.root) {
            None
        } else {
            current.map(|idx| &self.inner[idx])
        }
    }

    pub(crate) fn package_at_path(&self, path: &Path) -> Option<Package> {
        Some(self.node_at_path(path)?.package.clone())
    }

    pub(crate) fn find_by_name(
        &self,
        parent: NodeIndex,
        name: &UniCase<String>,
    ) -> Result<Option<NodeIndex>, NodeMaintainerError> {
        Ok(self.node_parent_iter(parent).find_map(|node| {
            if node.children.contains_key(name) {
                Some(node.children[name])
            } else {
                None
            }
        }))
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

        // Verify that depencies are satisfied by the logical hierarchy.
        for dependent in self.inner.node_weights() {
            for (dep_name, edge_idx) in &dependent.dependencies {
                let edge = &self.inner[*edge_idx];

                if let Some(dep_idx) = self.resolve_dep(dependent.idx, dep_name) {
                    let dependency = &self.inner[dep_idx];

                    if !dependency.package.resolved().satisfies(&edge.requested)? {
                        return Err(GraphValidationError(format!(
                            "Dependency {:?} does not satisfy requirement {:?} from {:?}",
                            dependency.package.resolved(),
                            edge.requested,
                            dependent.package.resolved(),
                        )));
                    }
                } else {
                    return Err(GraphValidationError(format!(
                        "Dependency {:?} {:?} not reachable from {:?}",
                        dep_name,
                        edge.requested,
                        dependent.package.resolved(),
                    )));
                }
            }
        }

        Ok(())
    }

    pub(crate) fn node_lockfile_node(
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
        let dependencies = node.dependencies.iter().map(|(name, edge_idx)| {
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

pub(crate) struct NodeParentIterator<'a> {
    graph: &'a Graph,
    current: Option<NodeIndex>,
}

impl<'a> Iterator for NodeParentIterator<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(idx) = self.current {
            let res = &self.graph[idx];
            self.current = res.parent;
            Some(res)
        } else {
            None
        }
    }
}
