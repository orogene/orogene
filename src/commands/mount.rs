use std::time::Duration;

use clap::Args;
use miette::Result;

use crate::commands::OroCommand;

/// Ping the registry.
#[derive(Debug, Args)]
pub struct MountCmd {}

#[async_trait::async_trait]
impl OroCommand for MountCmd {
    async fn execute(self) -> Result<()> {
        #[cfg(target_os = "macos")]
        alabaster::macos::init().await?;
        async_std::task::sleep(Duration::from_secs(30)).await;
        Ok(())
    }
}
