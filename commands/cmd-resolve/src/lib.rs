use std::path::PathBuf;

use async_trait::async_trait;
use clap::Args;
use miette::{IntoDiagnostic, Result};
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
        NodeMaintainerOptions::new()
            .registry(self.registry)
            .resolve(root.canonicalize().into_diagnostic()?.to_string_lossy())
            .await?
            .write_lockfile(root.join("package-lock.kdl"))
            .await?;
        Ok(())
    }
}
