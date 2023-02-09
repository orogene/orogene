use std::collections::BTreeMap;

use nassun::package::Package;
use oro_common::CorgiManifest;
use petgraph::stable_graph::{EdgeIndex, NodeIndex};
use unicase::UniCase;

use crate::Graph;

#[derive(Debug, Clone)]
pub struct Node {
    /// Index of this Node inside its [`Graph`].
    pub(crate) idx: NodeIndex,
    /// Resolved [`Package`] for this Node.
    pub(crate) package: Package,
    /// Resolved [`CorgiManifest`] for this Node.
    pub(crate) manifest: CorgiManifest,
    /// Quick index back to this Node's [`Graph`]'s root Node.
    pub(crate) root: NodeIndex,
    /// Name-indexed map of outgoing [`crate::Edge`]s from this Node.
    pub(crate) dependencies: BTreeMap<UniCase<String>, EdgeIndex>,
    /// Parent, if any, of this Node in the logical filesystem hierarchy.
    pub(crate) parent: Option<NodeIndex>,
    /// Children of this node in the logical filesystem hierarchy. These are
    /// not necessarily dependencies, and this Node's dependencies may not all
    /// be in this HashMap.
    pub(crate) children: BTreeMap<UniCase<String>, NodeIndex>,
}

impl Node {
    pub(crate) fn new(package: Package, manifest: CorgiManifest) -> Self {
        Self {
            package,
            manifest,
            idx: NodeIndex::new(0),
            root: NodeIndex::new(0),
            parent: None,
            children: BTreeMap::new(),
            dependencies: BTreeMap::new(),
        }
    }

    /// This Node's depth in the logical filesystem hierarchy.
    pub(crate) fn depth(&self, graph: &Graph) -> usize {
        graph.node_parent_iter(self.idx).count() - 1
    }
}
