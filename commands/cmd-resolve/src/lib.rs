use std::{fs, path::PathBuf};

use async_trait::async_trait;
use clap::Args;
use miette::{Context, IntoDiagnostic, Result};
use node_maintainer::NodeMaintainerOptions;
use oro_command::OroCommand;
use oro_config::OroConfigLayer;
use url::Url;

#[derive(Debug, Args, OroConfigLayer)]
pub struct ResolveCmd {
    /// Default registry.
    #[arg(default_value = "https://registry.npmjs.org", long)]
    registry: Url,

    #[clap(from_global)]
    json: bool,

    #[clap(from_global)]
    quiet: bool,

    #[clap(from_global)]
    root: Option<PathBuf>,
}

#[async_trait]
impl OroCommand for ResolveCmd {
    async fn execute(self) -> Result<()> {
        let root = self.root.unwrap_or_else(|| PathBuf::from("."));
        let mut nm = NodeMaintainerOptions::new().registry(self.registry);
        let lock_path = root.join("package-lock.kdl");
        if lock_path.exists() {
            let kdl = fs::read_to_string(&lock_path)
                .into_diagnostic()
                .wrap_err_with(|| {
                    format!("Failed to read lockfile at {}", lock_path.to_string_lossy())
                })?;
            nm = nm.kdl_lock(kdl).wrap_err_with(
                || format!("Failed to parse lockfile at {}", lock_path.to_string_lossy()),
            )?;
        }
        nm.resolve_spec(root.canonicalize().into_diagnostic()?.to_string_lossy())
            .await?
            .write_lockfile(root.join("package-lock.kdl"))
            .await?;
        Ok(())
    }
}
