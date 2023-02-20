use std::collections::HashMap;

use nassun::Package;
use oro_common::{CorgiManifest, CorgiVersionMetadata};
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
    pub(crate) dependencies: HashMap<UniCase<String>, EdgeIndex>,
    /// Parent, if any, of this Node in the logical filesystem hierarchy.
    pub(crate) parent: Option<NodeIndex>,
    /// Children of this node in the logical filesystem hierarchy. These are
    /// not necessarily dependencies, and this Node's dependencies may not all
    /// be in this HashMap.
    pub(crate) children: HashMap<UniCase<String>, NodeIndex>,
    /// Tarball file size
    pub(crate) tarball_size: Option<u64>,
    /// The number of individual files in the tarball
    pub(crate) tarball_file_count: Option<u64>,
}

impl Node {
    pub(crate) fn new(package: Package, metadata: CorgiVersionMetadata) -> Self {
        Self {
            package,
            manifest: metadata.manifest,
            idx: NodeIndex::new(0),
            root: NodeIndex::new(0),
            parent: None,
            children: HashMap::new(),
            dependencies: HashMap::new(),
            tarball_size: metadata.dist.unpacked_size.and_then(|s| s.try_into().ok()),
            tarball_file_count: metadata.dist.file_count.and_then(|s| s.try_into().ok()),
        }
    }

    /// This Node's depth in the logical filesystem hierarchy.
    pub(crate) fn depth(&self, graph: &Graph) -> usize {
        graph.node_parent_iter(self.idx).count() - 1
    }
}
