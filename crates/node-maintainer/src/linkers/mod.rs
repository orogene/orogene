#[cfg(not(target_arch = "wasm32"))]
mod hoisted;
#[cfg(not(target_arch = "wasm32"))]
mod isolated;

#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
use hoisted::HoistedLinker;
#[cfg(not(target_arch = "wasm32"))]
use isolated::IsolatedLinker;

#[cfg(not(target_arch = "wasm32"))]
use crate::{
    graph::Graph, Lockfile, NodeMaintainerError, ProgressHandler, PruneProgress, ScriptLineHandler,
    ScriptStartHandler,
};

#[cfg(not(target_arch = "wasm32"))]
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
    Isolated(IsolatedLinker),
    #[cfg(not(target_arch = "wasm32"))]
    Hoisted(HoistedLinker),
    #[allow(dead_code)]
    Null,
}

impl Linker {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn isolated(opts: LinkerOptions) -> Self {
        Self::Isolated(IsolatedLinker(opts))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn hoisted(opts: LinkerOptions) -> Self {
        Self::Hoisted(HoistedLinker(opts))
    }

    #[allow(dead_code)]
    pub fn null() -> Self {
        Self::Null
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn prune(
        &self,
        #[allow(dead_code)] graph: &Graph,
    ) -> Result<usize, NodeMaintainerError> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Isolated(isolated) => isolated.prune(graph).await,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Hoisted(hoisted) => hoisted.prune(graph).await,
            Self::Null => Ok(0),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn extract(
        &self,
        #[allow(dead_code)] graph: &Graph,
    ) -> Result<usize, NodeMaintainerError> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Isolated(isolated) => isolated.extract(graph).await,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Hoisted(hoisted) => hoisted.extract(graph).await,
            Self::Null => Ok(0),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn rebuild(
        &self,
        #[allow(dead_code)] graph: &Graph,
        #[allow(dead_code)] ignore_scripts: bool,
    ) -> Result<(), NodeMaintainerError> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Isolated(isolated) => isolated.rebuild(graph, ignore_scripts).await,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Hoisted(hoisted) => hoisted.rebuild(graph, ignore_scripts).await,
            Self::Null => Ok(()),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn supports_reflink(src_dir: &Path, dest_dir: &Path) -> bool {
    let temp = match tempfile::NamedTempFile::new_in(src_dir) {
        Ok(t) => t,
        Err(e) => {
            tracing::debug!("error creating tempfile while checking for reflink support: {e}.");
            return false;
        }
    };
    match std::fs::write(&temp, "a") {
        Ok(_) => {}
        Err(e) => {
            tracing::debug!("error writing to tempfile while checking for reflink support: {e}.");
            return false;
        }
    };
    let tempdir = match tempfile::TempDir::new_in(dest_dir) {
        Ok(t) => t,
        Err(e) => {
            tracing::debug!(
                "error creating destination tempdir while checking for reflink support: {e}."
            );
            return false;
        }
    };
    let supports_reflink = reflink::reflink(temp.path(), tempdir.path().join("b"))
        .map(|_| true)
        .map_err(|e| {
            tracing::debug!(
                "reflink support check failed. Files will be hard linked or copied. ({e})"
            );
            e
        })
        .unwrap_or(false);

    if supports_reflink {
        tracing::debug!("Verified reflink support. Extracted data will use copy-on-write reflinks instead of hard links or full copies.")
    }

    supports_reflink
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn link_bin(from: &Path, to: &Path) -> Result<(), NodeMaintainerError> {
    #[cfg(windows)]
    oro_shim_bin::shim_bin(from, to)?;
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = from.metadata()?;
        let mut perms = meta.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(from, perms)?;
        let relative = pathdiff::diff_paths(from, to.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(relative, to)?;
    }
    Ok(())
}
