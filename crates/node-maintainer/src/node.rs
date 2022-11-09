use std::collections::HashMap;

use nassun::Package;
use petgraph::stable_graph::{NodeIndex, EdgeIndex};
use unicase::UniCase;

use crate::Graph;

#[derive(Debug)]
pub struct Node {
    pub(crate) idx: NodeIndex,
    pub(crate) package: Package,
    pub(crate) root: NodeIndex,
    pub(crate) dependencies: HashMap<UniCase<String>, EdgeIndex>,
    pub(crate) parent: Option<NodeIndex>,
    pub(crate) children: HashMap<UniCase<String>, NodeIndex>,
}

impl Node {
    pub(crate) fn new(package: Package) -> Self {
        Self {
            idx: NodeIndex::new(0),
            package,
            root: NodeIndex::new(0),
            parent: None,
            children: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }

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
