#[cfg(not(target_arch = "wasm32"))]
mod hoisted;

use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use hoisted::HoistedLinker;

use crate::{
    graph::Graph, Lockfile, NodeMaintainerError, ProgressHandler, PruneProgress, ScriptLineHandler,
    ScriptStartHandler,
};

pub(crate) struct LinkerOptions {
    pub(crate) concurrency: usize,
    pub(crate) actual_tree: Option<Lockfile>,
    pub(crate) script_concurrency: usize,
    pub(crate) cache: Option<PathBuf>,
    pub(crate) prefer_copy: bool,
    pub(crate) validate: bool,
    pub(crate) root: PathBuf,
    pub(crate) on_prune_progress: Option<PruneProgress>,
    pub(crate) on_extract_progress: Option<ProgressHandler>,
    pub(crate) on_script_start: Option<ScriptStartHandler>,
    pub(crate) on_script_line: Option<ScriptLineHandler>,
}
pub(crate) enum Linker {
    #[cfg(not(target_arch = "wasm32"))]
    Hoisted(HoistedLinker),
    #[allow(dead_code)]
    Null,
}

impl Linker {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn hoisted(opts: LinkerOptions) -> Self {
        Self::Hoisted(HoistedLinker(opts))
    }

    #[allow(dead_code)]
    pub fn null() -> Self {
        Self::Null
    }

    pub async fn prune(&self, graph: &Graph) -> Result<usize, NodeMaintainerError> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Hoisted(hoisted) => hoisted.prune(graph).await,
            Self::Null => Ok(0),
        }
    }

    pub async fn extract(&self, graph: &Graph) -> Result<usize, NodeMaintainerError> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Hoisted(hoisted) => hoisted.extract(graph).await,
            Self::Null => Ok(0),
        }
    }

    pub async fn link_bins(&self, graph: &Graph) -> Result<usize, NodeMaintainerError> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Hoisted(hoisted) => hoisted.link_bins(graph).await,
            Self::Null => Ok(0),
        }
    }

    pub async fn rebuild(
        &self,
        graph: &Graph,
        ignore_scripts: bool,
    ) -> Result<(), NodeMaintainerError> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Hoisted(hoisted) => hoisted.rebuild(graph, ignore_scripts).await,
            Self::Null => Ok(()),
        }
    }

    pub async fn run_scripts(&self, graph: &Graph, event: &str) -> Result<(), NodeMaintainerError> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Hoisted(hoisted) => hoisted.run_scripts(graph, event).await,
            Self::Null => Ok(()),
        }
    }
}
