use std::collections::HashMap;

use nassun::Package;
use node_semver::Version;
use oro_package_spec::PackageSpec;
use petgraph::stable_graph::{EdgeIndex, NodeIndex};
use ssri::Integrity;
use unicase::UniCase;

use crate::Graph;

#[derive(Debug)]
pub struct Node {
    /// Index of this Node inside its [`Graph`].
    pub(crate) idx: NodeIndex,
    /// Resolved [`Package`] for this Node.
    pub(crate) package: Package,
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
    /// Statically-cached minimal package metadata for this Node.
    pub(crate) resolved_metadata: ResolvedMetadata,
}

impl Node {
    pub(crate) fn new(package: Package) -> Self {
        Self {
            package,
            idx: NodeIndex::new(0),
            root: NodeIndex::new(0),
            parent: None,
            children: HashMap::new(),
            dependencies: HashMap::new(),
            resolved_metadata: ResolvedMetadata::default(),
        }
    }

    /// This Node's depth in the logical filesystem hierarchy.
    pub(crate) fn depth(&self, graph: &Graph) -> usize {
        let mut depth = 0;
        let mut current = self.parent;
        while let Some(idx) = current {
            depth += 1;
            current = graph.inner[idx].parent;
        }
        depth
    }
}

/// Contains statically-available metadata for a [`Node`]. In essence, this
/// represents stuff we can put in a lockfile, that will help us build and
/// resolve a package graph without needing to make any Nassun requests.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedMetadata {
    pub(crate) name: UniCase<String>,
    pub(crate) path: Vec<UniCase<String>>,
    pub(crate) version: Option<Version>,
    pub(crate) resolved: Option<String>,
    pub(crate) integrity: Option<Integrity>,
    pub(crate) dependencies: HashMap<UniCase<String>, PackageSpec>,
    pub(crate) dev_dependencies: HashMap<UniCase<String>, PackageSpec>,
    pub(crate) peer_dependencies: HashMap<UniCase<String>, PackageSpec>,
    pub(crate) optional_dependencies: HashMap<UniCase<String>, PackageSpec>,
}
