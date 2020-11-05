use std::env;
use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use clap::Clap;
use node_maintainer::NodeMaintainerOptions;
use oro_command::OroCommand;
use oro_config::OroConfigLayer;
use url::Url;

#[derive(Debug, Clap, OroConfigLayer)]
pub struct PrimeCmd {
    #[clap(from_global)]
    root: Option<PathBuf>,
    #[clap(
        about = "Registry to install from.",
        default_value = "https://registry.npmjs.org",
        long
    )]
    registry: Url,
    #[clap(from_global)]
    json: bool,
    #[clap(from_global)]
    quiet: bool,
}

#[async_trait]
impl OroCommand for PrimeCmd {
    async fn execute(self) -> Result<()> {
        let cwd = env::current_dir()?;
        let root = self
            .root
            .unwrap_or_else(|| oro_pkg_root::pkg_root(&cwd).unwrap_or(cwd));
        let mut nm = NodeMaintainerOptions::new()
            .registry(self.registry)
            .path(root.clone())
            .init(root.display().to_string())
            .await?;
        nm.resolve().await?;
        nm.render();
        Ok(())
    }
}
