use std::{fs, path::PathBuf};

use async_trait::async_trait;
use clap::Args;
use miette::{Context, IntoDiagnostic, Result};
use node_maintainer::NodeMaintainerOptions;
use oro_config::OroConfigLayer;
use url::Url;

use crate::commands::OroCommand;

#[derive(Debug, Args, OroConfigLayer)]
pub struct RestoreCmd {
    #[clap(from_global)]
    registry: Option<Url>,

    #[clap(from_global)]
    json: bool,

    #[clap(from_global)]
    quiet: bool,

    #[clap(from_global)]
    root: Option<PathBuf>,

    #[clap(from_global)]
    cache: Option<PathBuf>,

    /// Prefer copying files over hard linking them.
    ///
    /// On filesystems that don't support copy-on-write/reflinks (usually NTFS
    /// or ext4), orogene defaults to hard linking package files from a
    /// centralized cache. As such, this can cause global effects if a file
    /// inside a node_modules is modified, where other projects that have
    /// installed that same file will see those modifications.
    ///
    /// In order to prevent this, you can use this flag to force orogene to
    /// always copy files, at a performance cost.
    #[arg(short, long)]
    prefer_copy: bool,

    /// Validate the integrity of installed files.
    ///
    /// When this is true, orogene will verify all files extracted from the
    /// cache, as well as verify that any files in the existing `node_modules`
    /// are unmodified. If verification fails, the packages will be
    /// reinstalled.
    #[arg(short, long)]
    validate: bool,
}

#[async_trait]
impl OroCommand for RestoreCmd {
    async fn execute(self) -> Result<()> {
        let root = self
            .root
            .expect("root should've been set by global defaults");
        let mut nm = NodeMaintainerOptions::new();
        nm = nm
            .progress_bar(true)
            .prefer_copy(self.prefer_copy)
            .validate(self.validate);
        if let Some(registry) = self.registry {
            nm = nm.registry(registry);
        }
        if let Some(cache) = self.cache {
            nm = nm.cache(cache);
        }
        let lock_path = root.join("package-lock.kdl");
        if lock_path.exists() {
            let kdl = fs::read_to_string(&lock_path)
                .into_diagnostic()
                .wrap_err_with(|| {
                    format!("Failed to read lockfile at {}", lock_path.to_string_lossy())
                })?;
            nm = nm.kdl_lock(kdl).wrap_err_with(|| {
                format!(
                    "Failed to parse lockfile at {}",
                    lock_path.to_string_lossy()
                )
            })?;
        }
        let lock_path = root.join("package-lock.json");
        if lock_path.exists() {
            let json = fs::read_to_string(&lock_path)
                .into_diagnostic()
                .wrap_err_with(|| {
                    format!("Failed to read lockfile at {}", lock_path.to_string_lossy())
                })?;
            nm = nm.npm_lock(json).wrap_err_with(|| {
                format!(
                    "Failed to parse NPM package lockfile at {}",
                    lock_path.to_string_lossy()
                )
            })?;
        }
        let resolved_nm = nm
            .resolve_spec(root.canonicalize().into_diagnostic()?.to_string_lossy())
            .await?;
        resolved_nm.extract_to(&root).await?;
        resolved_nm
            .write_lockfile(root.join("package-lock.kdl"))
            .await?;
        Ok(())
    }
}
