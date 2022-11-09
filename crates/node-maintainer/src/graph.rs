use std::ops::{Index, IndexMut};

use petgraph::{
    dot::Dot,
    stable_graph::{NodeIndex, StableGraph},
};
use unicase::UniCase;

use crate::{Edge, Node, NodeMaintainerError};

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

impl Graph {
    pub fn render(&self) -> String {
        format!("{:?}", Dot::new(&self.inner))
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
}
