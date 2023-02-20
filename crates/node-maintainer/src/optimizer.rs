use petgraph::algo::dominators;
use petgraph::algo::DfsSpace;
use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::visit::{EdgeRef, Visitable};
use petgraph::Direction;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::ops::{Index, IndexMut};

const MAX_PROMOTE_ITERATIONS: usize = 10;

pub(crate) trait Package {
    fn name(&self) -> &str;
}

#[derive(Debug, Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub(crate) struct TreeIndex(usize);

#[derive(Debug, Clone)]
pub(crate) struct TreeNode<P: Package + Clone> {
    pub(crate) idx: TreeIndex,
    pub(crate) graph_idx: NodeIndex,
    pub(crate) package: P,
    pub(crate) parent: Option<TreeIndex>,
    pub(crate) children: BTreeMap<String, TreeIndex>,

    dependents: HashSet<TreeIndex>,
    conflicts: BTreeMap<String, Vec<TreeIndex>>,
}

#[derive(Debug, Clone)]
pub(crate) struct Tree<P: Package + Clone> {
    pub(crate) root: TreeIndex,
    inner: HashMap<TreeIndex, TreeNode<P>>,
    node_count: usize,
}

impl<P: Package + Clone> Index<TreeIndex> for Tree<P> {
    type Output = TreeNode<P>;

    fn index(&self, idx: TreeIndex) -> &TreeNode<P> {
        &self.inner[&idx]
    }
}

impl<P: Package + Clone> IndexMut<TreeIndex> for Tree<P> {
    fn index_mut(&mut self, idx: TreeIndex) -> &mut TreeNode<P> {
        self.inner.get_mut(&idx).expect("lookup failure")
    }
}

type StableDfsSpace<P, E> = DfsSpace<NodeIndex, <StableGraph<P, E> as Visitable>::Map>;

impl<P: Package + Clone> Tree<P> {
    pub(crate) fn build<E>(graph: &StableGraph<P, E>, root: NodeIndex) -> Self {
        let mut tree = Tree {
            root: TreeIndex(0),
            inner: HashMap::new(),
            node_count: 0,
        };

        let dominators = dominators::simple_fast(graph, root);
        let mut idx_converter = HashMap::new();

        tree.build_subtree(graph, &dominators, root, None, &mut idx_converter);
        tree.init_dependents(graph, tree.root, &idx_converter);

        let mut dfs = DfsSpace::new(graph);
        tree.resolve_conflicts(graph, &mut dfs, tree.root);

        for _ in 0..MAX_PROMOTE_ITERATIONS {
            if !tree.promote_leafs(graph, tree.root) {
                break;
            }
        }

        tree
    }

    pub(crate) fn nodes(&self) -> TreeNodeIterator<'_, P> {
        let mut queue = VecDeque::new();
        queue.push_back(self.root);
        TreeNodeIterator { tree: self, queue }
    }

    fn build_subtree<E>(
        &mut self,
        graph: &StableGraph<P, E>,
        dominators: &dominators::Dominators<NodeIndex>,
        root: NodeIndex,
        parent: Option<TreeIndex>,
        idx_converter: &mut HashMap<NodeIndex, TreeIndex>,
    ) -> bool {
        if idx_converter.contains_key(&root) {
            return false;
        }

        let idx = TreeIndex(self.node_count);
        self.node_count += 1;
        idx_converter.insert(root, idx);

        let mut conflicts: BTreeMap<String, Vec<TreeIndex>> = BTreeMap::new();
        for child_idx in dominators.immediately_dominated_by(root) {
            if !self.build_subtree(graph, dominators, child_idx, Some(idx), idx_converter) {
                // Detected loop, break it. First occurence of the package in
                // the loop would be used throughout the loop.
                continue;
            }

            let child_name = graph[child_idx].name().to_string();
            let child_idx = idx_converter[&child_idx];
            if let Some(versions) = conflicts.get_mut(&child_name) {
                versions.push(child_idx);
            } else {
                conflicts.insert(child_name, vec![child_idx]);
            }
        }

        self.inner.insert(
            idx,
            TreeNode {
                idx,
                graph_idx: root,
                package: graph[root].clone(),
                parent,
                // Will be filled later
                dependents: HashSet::new(),
                conflicts,
                children: BTreeMap::new(),
            },
        );

        true
    }

    fn init_dependents<E>(
        &mut self,
        graph: &StableGraph<P, E>,
        root: TreeIndex,
        idx_converter: &HashMap<NodeIndex, TreeIndex>,
    ) {
        let tree_node = &mut self[root];
        tree_node.dependents.extend(
            graph
                .edges_directed(tree_node.graph_idx, Direction::Incoming)
                .map(|e| idx_converter[&e.source()]),
        );

        // Satisfy borrow checker
        let conflicts = tree_node
            .conflicts
            .values()
            .flat_map(|v| v.iter())
            .cloned()
            .collect::<Vec<_>>();
        for tree_idx in conflicts {
            self.init_dependents(graph, tree_idx, idx_converter);
        }
    }

    fn resolve_conflicts<E>(
        &mut self,
        graph: &StableGraph<P, E>,
        dfs: &mut StableDfsSpace<P, E>,
        root: TreeIndex,
    ) {
        let mut queue = Vec::new();

        // We need to have two mutable references to node's conflicts so
        // unfortunately this copy of conflicts keys is needed.
        let child_names = self.inner[&root]
            .conflicts
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for name in child_names {
            // Package that conflicts with root's dependencies must be
            // immediately promoted.
            let hard_conflicts = self.inner[&root].conflicts[&name]
                .iter()
                .cloned()
                .enumerate()
                .filter(|(_, idx)| self.is_incompatible(graph, root, *idx))
                .collect::<Vec<_>>();
            for (i, conflict_idx) in hard_conflicts {
                self[root]
                    .conflicts
                    .get_mut(&name)
                    .expect("child package")
                    .remove(i);
                self.demote(graph, dfs, root, conflict_idx);
            }

            while self.inner[&root].conflicts[&name].len() > 1 {
                // Select conflicting package with less dependent packages that
                // are not dependencies of the root.
                let (i, least_used) = self.inner[&root].conflicts[&name]
                    .iter()
                    .cloned()
                    .enumerate()
                    .filter(|(_, idx)| !self.inner[idx].dependents.contains(&root))
                    .min_by_key(|(_, idx)| self.inner[idx].dependents.len())
                    .expect("least used duplicate");

                // Remove package from conflicts
                self[root]
                    .conflicts
                    .get_mut(&name)
                    .expect("child package")
                    .remove(i);

                // Demote package into node's children that are also
                // ancestors of the `least_used` (i.e. the subtrees that use
                // `least_used`).
                self.demote(graph, dfs, root, least_used);
            }

            let conflicts = &self.inner[&root].conflicts[&name];
            if !conflicts.is_empty() {
                assert_eq!(conflicts.len(), 1);
                queue.push(conflicts[0]);
            }
        }

        // Populate `children` by draining `conflicts`
        {
            let root = &mut self[root];

            while let Some((name, conflicts)) = root.conflicts.pop_last() {
                if !conflicts.is_empty() {
                    assert_eq!(conflicts.len(), 1);
                    root.children.insert(name, conflicts[0]);
                }
            }
        }

        // Recurse
        for child_idx in queue {
            self.resolve_conflicts(graph, dfs, child_idx);
        }
    }

    fn demote<E>(
        &mut self,
        graph: &StableGraph<P, E>,
        dfs: &mut StableDfsSpace<P, E>,
        root: TreeIndex,
        dep: TreeIndex,
    ) {
        // Find conflicts of `root` that are still ancestors of `dep`
        let targets = self.inner[&root]
            .conflicts
            .values()
            .flatten()
            .cloned()
            .filter(|&child| self.is_ancestor(graph, dfs, child, dep))
            .collect::<Vec<_>>();

        // Put a copy of dep into each such child.
        for child in targets {
            let mut dep = self.clone_subtree(dep, Some(child));
            let name = dep.package.name().to_string();

            // Make sure that dependents are within the child's subtree
            dep.dependents
                .retain(|&dependent| self.is_ancestor(graph, dfs, child, dependent));

            let child = &mut self[child];
            if let Some(versions) = child.conflicts.get_mut(&name) {
                versions.push(dep.idx);
            } else {
                child.conflicts.insert(name, vec![dep.idx]);
            }

            self.inner.insert(dep.idx, dep);
        }

        // Finally remove original subtree from the graph
        self.remove_subtree(dep);
    }

    fn clone_subtree(&mut self, root: TreeIndex, parent: Option<TreeIndex>) -> TreeNode<P> {
        let idx = TreeIndex(self.node_count);
        self.node_count += 1;
        let mut clone = self.inner[&root].clone();
        clone.idx = idx;
        clone.parent = parent;

        // Clone each conflict
        for versions in clone.conflicts.values_mut() {
            for child in versions.iter_mut() {
                let cloned_child = self.clone_subtree(*child, Some(idx));
                *child = cloned_child.idx;
                self.inner.insert(cloned_child.idx, cloned_child);
            }
        }

        clone
    }

    fn remove_subtree(&mut self, root: TreeIndex) {
        let root = self.inner.remove(&root).expect("removed dependency");

        for versions in root.conflicts.into_values() {
            for child in versions.into_iter() {
                self.remove_subtree(child);
            }
        }
    }

    fn is_ancestor<E>(
        &self,
        graph: &StableGraph<P, E>,
        dfs: &mut StableDfsSpace<P, E>,
        ancestor: TreeIndex,
        node: TreeIndex,
    ) -> bool {
        petgraph::algo::has_path_connecting(
            graph,
            self.inner[&ancestor].graph_idx,
            self.inner[&node].graph_idx,
            Some(dfs),
        )
    }

    fn is_incompatible<E>(
        &self,
        graph: &StableGraph<P, E>,
        parent: TreeIndex,
        child: TreeIndex,
    ) -> bool {
        graph
            .edges_directed(self.inner[&parent].graph_idx, Direction::Outgoing)
            .any(|e| {
                e.target() != self.inner[&child].graph_idx
                    && graph[e.target()].name() == self.inner[&child].package.name()
            })
    }

    fn promote_leafs<E>(&mut self, graph: &StableGraph<P, E>, root: TreeIndex) -> bool {
        let mut changes = false;

        // Clear unused data
        self[root].dependents.clear();

        // Recurse and promote dependencies in children
        let children = self.inner[&root]
            .children
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for child_name in children {
            if self.promote_leafs(graph, self.inner[&root].children[&child_name]) {
                changes = true;
            }
        }

        // Note that we need a fresh list of children here since it could have
        // changed.
        if let Some(parent) = self.inner[&root].parent {
            let children = self.inner[&root]
                .children
                .clone()
                .into_iter()
                .collect::<Vec<_>>();

            // Now try to promote each leaf child.
            for (child_name, child_idx) in children.iter() {
                let is_leaf = self.inner[child_idx].children.is_empty();
                if !is_leaf {
                    continue;
                }

                // Check if the leaf child has dependencies under the root. If
                // yes - it won't be able to require them from the parent so
                // we cannot promote it.
                let has_local_deps = children.iter().any(|(_, other_idx)| {
                    // Other child already promoted!
                    if !self.inner.contains_key(other_idx) {
                        return false;
                    }

                    graph.contains_edge(
                        self.inner[child_idx].graph_idx,
                        self.inner[other_idx].graph_idx,
                    )
                });
                if has_local_deps {
                    continue;
                }

                // Check if parent requires a different version of the package
                if self.is_incompatible(graph, parent, *child_idx) {
                    continue;
                }

                // Check if parent already has a child with the same version
                if let Some(other_child) = self.inner[&parent].children.get(child_name) {
                    // If yes, but different version - not optimizable
                    if self.inner[other_child].graph_idx != self.inner[child_idx].graph_idx {
                        continue;
                    }

                    // If yes - the leaf is redundant
                    self[root]
                        .children
                        .remove(child_name)
                        .expect("child to be removed");
                    self.remove_subtree(*child_idx);
                } else {
                    // Move child from root to root's parent.
                    self[root]
                        .children
                        .remove(child_name)
                        .expect("child to be removed");
                    self[parent].children.insert(child_name.clone(), *child_idx);

                    // Change child's parent
                    self[*child_idx].parent = Some(parent);
                }

                changes = true;
            }
        }

        changes
    }
}

pub(crate) struct TreeNodeIterator<'a, P: Package + Clone> {
    tree: &'a Tree<P>,
    queue: VecDeque<TreeIndex>,
}

impl<'a, P: Package + Clone> Iterator for TreeNodeIterator<'a, P> {
    type Item = &'a TreeNode<P>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(idx) = self.queue.pop_front() {
            let res = &self.tree[idx];
            self.queue.extend(res.children.values());
            Some(res)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct Node {
        name: String,
        version: String,
    }

    #[derive(Debug, Clone)]
    struct Edge {}

    impl Package for Node {
        fn name(&self) -> &str {
            &self.name
        }
    }

    impl Node {
        fn new(name: &str, version: &str) -> Self {
            Self {
                name: name.to_string(),
                version: version.to_string(),
            }
        }
    }

    fn render_tree(tree: &Tree<Node>) -> Vec<(String, Vec<String>)> {
        let mut res = Vec::new();
        for node in tree.nodes() {
            let mut children = node
                .children
                .values()
                .map(|&idx| format!("{}@{}", tree[idx].package.name, tree[idx].package.version))
                .collect::<Vec<_>>();
            children.sort();

            // verify that parent is in the tree
            if let Some(parent) = node.parent {
                assert!(tree.inner.contains_key(&parent));

                // Verify that this node is a children of its parent
                tree.inner[&parent]
                    .children
                    .values()
                    .find(|&&child_idx| child_idx == node.idx)
                    .expect("child to be present in parent");
            }

            res.push((
                format!("{}@{}", node.package.name, node.package.version),
                children,
            ));
        }
        res
    }

    #[test]
    fn it_promotes_shared_dependencies() {
        let mut graph = StableGraph::new();

        let root = graph.add_node(Node::new("root", "1.0.0"));
        let a = graph.add_node(Node::new("a", "1.0.0"));
        graph.add_edge(root, a, Edge {});
        let b = graph.add_node(Node::new("b", "1.0.0"));
        graph.add_edge(root, b, Edge {});
        let shared = graph.add_node(Node::new("shared", "1.0.0"));
        graph.add_edge(a, shared, Edge {});
        graph.add_edge(b, shared, Edge {});

        let tree = Tree::build(&graph, root);
        assert_eq!(tree.inner.len(), 4);

        assert_eq!(
            render_tree(&tree),
            vec![
                (
                    "root@1.0.0".into(),
                    vec!["a@1.0.0".into(), "b@1.0.0".into(), "shared@1.0.0".into()]
                ),
                ("a@1.0.0".into(), vec![]),
                ("b@1.0.0".into(), vec![]),
                ("shared@1.0.0".into(), vec![]),
            ],
        );
    }

    #[test]
    fn it_demotes_conflicts() {
        let mut graph = StableGraph::new();

        let root = graph.add_node(Node::new("root", "1.0.0"));
        let a = graph.add_node(Node::new("a", "1.0.0"));
        graph.add_edge(root, a, Edge {});
        let b = graph.add_node(Node::new("b", "1.0.0"));
        graph.add_edge(root, b, Edge {});
        let shared = graph.add_node(Node::new("shared", "1.0.0"));
        graph.add_edge(a, shared, Edge {});
        graph.add_edge(b, shared, Edge {});
        let leaf = graph.add_node(Node::new("leaf", "1.0.0"));
        graph.add_edge(shared, leaf, Edge {});
        let shared2 = graph.add_node(Node::new("shared", "2.0.0"));
        graph.add_edge(root, shared2, Edge {});

        let tree = Tree::build(&graph, root);
        assert_eq!(tree.inner.len(), 7);

        assert_eq!(
            render_tree(&tree),
            vec![
                (
                    "root@1.0.0".into(),
                    vec![
                        "a@1.0.0".into(),
                        "b@1.0.0".into(),
                        "leaf@1.0.0".into(),
                        "shared@2.0.0".into()
                    ]
                ),
                ("a@1.0.0".into(), vec!["shared@1.0.0".into()]),
                ("b@1.0.0".into(), vec!["shared@1.0.0".into()]),
                ("leaf@1.0.0".into(), vec![]),
                ("shared@2.0.0".into(), vec![]),
                ("shared@1.0.0".into(), vec![]),
                ("shared@1.0.0".into(), vec![]),
            ],
        );
    }

    #[test]
    fn it_doesnt_create_conflicts_after_demotion() {
        let mut graph = StableGraph::new();

        let root = graph.add_node(Node::new("root", "1.0.0"));

        let a = graph.add_node(Node::new("a", "1.0.0"));
        let b = graph.add_node(Node::new("b", "1.0.0"));
        let c = graph.add_node(Node::new("c", "1.0.0"));
        let d = graph.add_node(Node::new("d", "1.0.0"));

        let s1 = graph.add_node(Node::new("s", "1.0.0"));
        let s2 = graph.add_node(Node::new("s", "2.0.0"));

        graph.add_edge(root, a, Edge {});
        graph.add_edge(root, b, Edge {});
        graph.add_edge(root, c, Edge {});

        graph.add_edge(c, d, Edge {});

        // A and D depend on old version of s
        graph.add_edge(a, s1, Edge {});
        graph.add_edge(d, s1, Edge {});

        // B and C on new version
        graph.add_edge(b, s2, Edge {});
        graph.add_edge(c, s2, Edge {});

        let tree = Tree::build(&graph, root);
        assert_eq!(tree.inner.len(), 8);

        assert_eq!(
            render_tree(&tree),
            vec![
                (
                    "root@1.0.0".into(),
                    vec![
                        "a@1.0.0".into(),
                        "b@1.0.0".into(),
                        "c@1.0.0".into(),
                        "d@1.0.0".into(),
                        "s@1.0.0".into(),
                    ]
                ),
                ("a@1.0.0".into(), vec![]),
                ("b@1.0.0".into(), vec!["s@2.0.0".into()]),
                ("c@1.0.0".into(), vec!["s@2.0.0".into()]),
                ("d@1.0.0".into(), vec![]),
                ("s@1.0.0".into(), vec![]),
                ("s@2.0.0".into(), vec![]),
                ("s@2.0.0".into(), vec![]),
            ],
        );
    }

    #[test]
    fn it_doesnt_promote_past_dependencies() {
        let mut graph = StableGraph::new();

        let root = graph.add_node(Node::new("root", "1.0.0"));
        let a = graph.add_node(Node::new("a", "1.0.0"));
        graph.add_edge(root, a, Edge {});
        let b = graph.add_node(Node::new("b", "1.0.0"));
        graph.add_edge(a, b, Edge {});
        let c1 = graph.add_node(Node::new("c", "1.0.0"));
        graph.add_edge(b, c1, Edge {});
        let c2 = graph.add_node(Node::new("c", "2.0.0"));
        graph.add_edge(root, c2, Edge {});

        let tree = Tree::build(&graph, root);
        assert_eq!(tree.inner.len(), 5);

        assert_eq!(
            render_tree(&tree),
            vec![
                (
                    "root@1.0.0".into(),
                    vec!["a@1.0.0".into(), "c@2.0.0".into(),]
                ),
                ("a@1.0.0".into(), vec!["b@1.0.0".into(), "c@1.0.0".into()]),
                ("c@2.0.0".into(), vec![]),
                ("b@1.0.0".into(), vec![]),
                ("c@1.0.0".into(), vec![]),
            ],
        );
    }

    #[test]
    fn it_promotes_leaves_repeatedly() {
        let mut graph = StableGraph::new();

        let root = graph.add_node(Node::new("a", "1.0.0"));
        let b = graph.add_node(Node::new("b", "2.0.0"));
        graph.add_edge(root, b, Edge {});
        let c = graph.add_node(Node::new("c", "3.0.0"));
        graph.add_edge(b, c, Edge {});
        let d = graph.add_node(Node::new("d", "4.0.0"));
        graph.add_edge(b, d, Edge {});
        let c2 = graph.add_node(Node::new("c", "5.0.0"));
        graph.add_edge(d, c2, Edge {});

        let tree = Tree::build(&graph, root);
        assert_eq!(tree.inner.len(), 5);

        assert_eq!(
            render_tree(&tree),
            vec![
                ("a@1.0.0".into(), vec!["b@2.0.0".into(), "c@3.0.0".into(),]),
                ("b@2.0.0".into(), vec!["d@4.0.0".into()]),
                ("c@3.0.0".into(), vec![]),
                ("d@4.0.0".into(), vec!["c@5.0.0".into()]),
                ("c@5.0.0".into(), vec![]),
            ],
        );
    }

    #[test]
    fn it_doesnt_substitute_deps() {
        let mut graph = StableGraph::new();

        let root = graph.add_node(Node::new("root", "1.0.0"));
        let a = graph.add_node(Node::new("a", "1.0.0"));
        graph.add_edge(root, a, Edge {});
        let b = graph.add_node(Node::new("b", "1.0.0"));
        graph.add_edge(a, b, Edge {});
        // If we promote "c1" to root - we shouldn't promote "c2" to "a" because
        // "a" depends on "c1"
        let c1 = graph.add_node(Node::new("c", "1.0.0"));
        graph.add_edge(a, c1, Edge {});
        let c2 = graph.add_node(Node::new("c", "2.0.0"));
        graph.add_edge(b, c2, Edge {});

        let tree = Tree::build(&graph, root);
        assert_eq!(tree.inner.len(), 5);

        assert_eq!(
            render_tree(&tree),
            vec![
                (
                    "root@1.0.0".into(),
                    vec!["a@1.0.0".into(), "c@1.0.0".into(),]
                ),
                ("a@1.0.0".into(), vec!["b@1.0.0".into()]),
                ("c@1.0.0".into(), vec![]),
                ("b@1.0.0".into(), vec!["c@2.0.0".into()]),
                ("c@2.0.0".into(), vec![]),
            ],
        );
    }
}
