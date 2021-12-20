use std::path::{Path, PathBuf};

use oro_manifest::OroManifest;
use oro_package_spec::PackageSpec;
use oro_torus::Torus;
use petgraph::stable_graph::StableGraph;

pub struct NodeMaintainerBuilder {
    path: Option<PathBuf>,
    torus: Option<Torus>,
}

impl NodeMaintainerBuilder {
    pub fn new() -> Self {
        Self {
            path: None,
            torus: None,
        }
    }

    pub fn path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = Some(path.as_ref().to_path_buf());
        self
    }

    // TODO: builder stuff for all Torus configs

    pub fn build(self) -> NodeMaintainer {
        NodeMaintainer {
            root_path: self
                .path
                .unwrap_or_else(|| std::env::current_dir().unwrap()),
            torus: self.torus.unwrap_or_default(),
            pet: StableGraph::new(),
        }
    }
}

impl Default for NodeMaintainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
pub struct NodeMaintainer {
    root_path: PathBuf,
    torus: Torus,
    pet: StableGraph<Node, Edge>,
}

struct Node {
    /// The name this dependency will be loaded as (usually the name it will
    /// have in node_modules)
    name: String,
    /// The package.json manifest for this dependency.
    manifest: OroManifest,
}

struct Edge {
    /// The name under which this dependency relationship appears in the dependent's package.json.
    name: String,
    /// Package spec for the dependency relationship.
    spec: PackageSpec,
    /// The type of dependency reference. (prod, dev, peer, optional)
    ty: EdgeType,
}

enum EdgeType {
    Prod,
    Dev,
    Peer,
    Optional,
}
