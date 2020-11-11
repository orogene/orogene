use std::collections::{HashSet, VecDeque};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use futures::{future, FutureExt};
use oro_classic_resolver::ClassicResolver;
use petgraph::dot::Dot;
use petgraph::stable_graph::{NodeIndex, StableGraph};
use rogga::{Package, PackageSpec, Rogga, RoggaOpts};
use url::Url;

use crate::error::{Error, Internal};

// Public so I don't get warnings about unused stuff right now
pub mod assignment;
pub mod error;
pub mod incompat;
pub mod partial_solution;
pub mod set_relation;
pub mod solver;
pub mod term;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepType {
    Prod,
    Dev,
    Opt,
    Peer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub requested: PackageSpec,
    pub dep_type: DepType,
}

#[derive(Clone, Default)]
pub struct NodeMaintainerOptions {
    registry: Option<Url>,
    path: Option<PathBuf>,
}

impl NodeMaintainerOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn registry(mut self, registry: Url) -> Self {
        self.registry = Some(registry);
        self
    }

    pub fn path(mut self, path: impl AsRef<Path>) -> Self {
        self.path = Some(path.as_ref().into());
        self
    }

    pub async fn init(self, request: impl AsRef<str>) -> Result<NodeMaintainer, Error> {
        let rogga = RoggaOpts::new()
            .use_corgi(true)
            .add_registry(
                "",
                self.registry
                    .unwrap_or_else(|| Url::parse("https://registry.npmjs.org").unwrap()),
            )
            .build();
        let mut graph = StableGraph::new();
        let current_dir = env::current_dir().to_internal()?;
        let cwd = self.path.unwrap_or(current_dir);
        let resolver = ClassicResolver::new();
        let root_dep = rogga
            .arg_request(request.as_ref(), &cwd)
            .await?
            .resolve_with(&resolver)
            .await?;
        let root = graph.add_node(root_dep);
        Ok(NodeMaintainer {
            cwd,
            rogga,
            resolver,
            root,
            graph,
        })
    }
}

pub struct NodeMaintainer {
    cwd: PathBuf,
    rogga: Rogga,
    resolver: ClassicResolver,
    root: NodeIndex,
    graph: StableGraph<Package, Dependency>,
}

impl NodeMaintainer {
    pub fn render(&self) {
        fs::write(
            self.cwd.join("graph.dot"),
            format!("{:?}", Dot::new(&self.graph)),
        )
        .expect("Failed to write rendered graph");
        println!("graph written to {}", self.cwd.join("graph.dot").display());
    }

    pub async fn resolve(&mut self) -> Result<(), Error> {
        let mut packages = Vec::new();
        let mut q = VecDeque::new();
        q.push_back(self.root);
        while let Some(package_idx) = q.pop_front() {
            let package = &self.graph[package_idx];
            let manifest = package.metadata().await?.manifest;
            let mut names = HashSet::new();
            for ((name, spec), dep_type) in manifest
                .optional_dependencies
                .iter()
                .map(|x| (x, DepType::Opt))
                .chain(manifest.dependencies.iter().map(|x| (x, DepType::Prod)))
                .chain(manifest.dev_dependencies.iter().map(|x| (x, DepType::Dev)))
                .chain(
                    manifest
                        .peer_dependencies
                        .iter()
                        .map(|x| (x, DepType::Peer)),
                )
            {
                if !names.contains(&name[..])
                    && (dep_type != DepType::Dev || package_idx != self.root)
                {
                    names.insert(&name[..]);
                    let request = self.rogga.dep_request(&name[..], &spec[..], &self.cwd)?;
                    packages.push(
                        request
                            .resolve_with(&self.resolver)
                            .map(|pkg| (pkg, dep_type)),
                    );
                }
            }
            for (package, dep_type) in future::join_all(packages.drain(..)).await {
                let package = package?;
                let requested = package.from.clone();
                let child_idx = self.graph.add_node(package);
                q.push_back(child_idx);
                self.graph.add_edge(
                    package_idx,
                    child_idx,
                    Dependency {
                        requested,
                        dep_type,
                    },
                );
            }
        }
        Ok(())
    }
}
