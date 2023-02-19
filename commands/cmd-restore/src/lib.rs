use std::{fs, path::PathBuf};

use async_trait::async_trait;
use clap::Args;
use miette::{Context, IntoDiagnostic, Result};
use node_maintainer::NodeMaintainerOptions;
use oro_command::OroCommand;
use oro_config::OroConfigLayer;
use url::Url;

#[derive(Debug, Args, OroConfigLayer)]
pub struct RestoreCmd {
    #[clap(from_global)]
    registry: Option<Url>,

    /// Apply experimental optimization technique to lockfile.
    #[arg(long)]
    optimize_lockfile: bool,

    #[clap(from_global)]
    json: bool,

    #[clap(from_global)]
    quiet: bool,

    #[clap(from_global)]
    root: Option<PathBuf>,

    #[clap(from_global)]
    cache: Option<PathBuf>,
}

#[async_trait]
impl OroCommand for RestoreCmd {
    async fn execute(self) -> Result<()> {
        let root = self
            .root
            .expect("root should've been set by global defaults");
        let mut nm = NodeMaintainerOptions::new();
        if let Some(registry) = self.registry {
            nm = nm.registry(registry);
        }
        if let Some(cache) = self.cache {
            nm = nm.cache(cache);
        }
        nm = nm.optimize(self.optimize_lockfile);
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
